use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::windows::named_pipe::{ServerOptions, ClientOptions, NamedPipeServer};
use tracing::{error, info, warn};
use chrono::Utc;
use uuid::Uuid;
use liem_common::{
    AppManifest, IpcEnvelope, EventBusChannel, EventPayload,
    SubscribePayload, RequestPayload, ResponsePayload, IpcError
};

// Helper framing functions using 4-byte big-endian length prefix
pub async fn write_frame<W>(writer: &mut W, data: &[u8]) -> Result<(), std::io::Error>
where
    W: AsyncWrite + Unpin,
{
    let len = data.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_frame<R>(reader: &mut R) -> Result<Vec<u8>, std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    
    // Restrict to 16MB maximum frame size
    if len > 16 * 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Frame length exceeds 16MB limit",
        ));
    }
    
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

fn generate_msg_id() -> String {
    Uuid::new_v4().to_string()
}

struct ClientConnection {
    app_id: String,
    manifest: AppManifest,
    sender: mpsc::UnboundedSender<IpcEnvelope>,
}

struct BrokerRegistry {
    clients: HashMap<String, ClientConnection>,
    subscribers: HashMap<String, Vec<String>>,
    pending_requests: HashMap<String, String>, // request_id -> requester_app_id
}

pub struct IpcBroker {
    pipe_name: String,
    registry: Arc<Mutex<BrokerRegistry>>,
}

impl IpcBroker {
    pub fn new(pipe_name: &str) -> Self {
        Self {
            pipe_name: pipe_name.to_string(),
            registry: Arc::new(Mutex::new(BrokerRegistry {
                clients: HashMap::new(),
                subscribers: HashMap::new(),
                pending_requests: HashMap::new(),
            })),
        }
    }

    pub async fn start(&self, mut shutdown_rx: oneshot::Receiver<()>) -> Result<(), std::io::Error> {
        let mut is_first = true;
        loop {
            let server_res = ServerOptions::new()
                .first_pipe_instance(is_first)
                .create(&self.pipe_name);
            
            let server = match server_res {
                Ok(s) => s,
                Err(e) => {
                    error!("[Broker] Failed to create named pipe instance: {}", e);
                    return Err(e);
                }
            };
            
            is_first = false;

            tokio::select! {
                res = server.connect() => {
                    if let Err(e) = res {
                        error!("[Broker] Connection connect error: {}", e);
                        continue;
                    }
                    let reg = Arc::clone(&self.registry);
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(server, reg).await {
                            error!("[Broker] Client session error: {}", e);
                        }
                    });
                }
                _ = &mut shutdown_rx => {
                    info!("[Broker] Shutting down named pipe listener");
                    break;
                }
            }
        }
        Ok(())
    }
}

async fn handle_client(
    stream: NamedPipeServer,
    registry: Arc<Mutex<BrokerRegistry>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut reader, mut writer) = tokio::io::split(stream);

    // 1. Handshake: first frame must be a CapabilityPublish containing AppManifest
    let bytes = read_frame(&mut reader).await?;
    let handshake: IpcEnvelope = serde_json::from_slice(&bytes)?;
    
    let (app_id, manifest) = match handshake.channel {
        EventBusChannel::Publish(ref payload) if payload.topic == "CapabilityPublish" => {
            let manifest: AppManifest = serde_json::from_value(payload.data.clone())?;
            manifest.validate()?;
            (handshake.sender_id.clone(), manifest)
        }
        _ => {
            return Err("First message must be a CapabilityPublish containing a valid AppManifest".into());
        }
    };
    
    let (tx, mut rx) = mpsc::unbounded_channel::<IpcEnvelope>();
    
    // Register client in the broker registry and broadcast system connection event
    let manifest_clone = manifest.clone();
    {
        let mut reg = registry.lock().await;
        if reg.clients.contains_key(&app_id) {
            warn!("Client '{}' reconnected, replacing previous connection", app_id);
        }
        reg.clients.insert(
            app_id.clone(),
            ClientConnection {
                app_id: app_id.clone(),
                manifest,
                sender: tx,
            },
        );
        
        // Broadcast the connection event to subscribers of "_system.client_connected"
        if let Some(sub_ids) = reg.subscribers.get("_system.client_connected").cloned() {
            let broadcast_msg = IpcEnvelope {
                sender_id: "broker".to_string(),
                message_id: generate_msg_id(),
                timestamp: Utc::now(),
                channel: EventBusChannel::Broadcast(EventPayload {
                    topic: "_system.client_connected".to_string(),
                    data: serde_json::to_value(&manifest_clone).unwrap(),
                }),
            };
            for sub_id in sub_ids {
                if sub_id != app_id {
                    if let Some(conn) = reg.clients.get(&sub_id) {
                        let _ = conn.sender.send(broadcast_msg.clone());
                    }
                }
            }
        }
    }
    
    // Spawn writer task for this client connection
    let writer_app_id = app_id.clone();
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let serialized = match serde_json::to_vec(&msg) {
                Ok(b) => b,
                Err(e) => {
                    error!("[Broker] Failed to serialize message to {}: {}", writer_app_id, e);
                    continue;
                }
            };
            if let Err(e) = write_frame(&mut writer, &serialized).await {
                error!("[Broker] Error writing to client {}: {}", writer_app_id, e);
                break;
            }
        }
    });
    
    // Reader loop for client connection
    let reader_app_id = app_id.clone();
    let reader_registry = Arc::clone(&registry);
    let reader_task = tokio::spawn(async move {
        loop {
            match read_frame(&mut reader).await {
                Ok(bytes) => {
                    let msg: IpcEnvelope = match serde_json::from_slice(&bytes) {
                        Ok(m) => m,
                        Err(e) => {
                            error!("[Broker] Invalid message from {}: {}", reader_app_id, e);
                            continue;
                        }
                    };
                    
                    if let Err(e) = route_message(&reader_registry, &reader_app_id, msg).await {
                        error!("[Broker] Routing error for {}: {}", reader_app_id, e);
                    }
                }
                Err(e) => {
                    info!("[Broker] Client {} disconnected: {}", reader_app_id, e);
                    break;
                }
            }
        }
    });

    tokio::select! {
        _ = writer_task => {}
        _ = reader_task => {}
    }

    // Cleanup registry on client disconnect
    {
        let mut reg = registry.lock().await;
        reg.clients.remove(&app_id);
        for subscribers in reg.subscribers.values_mut() {
            subscribers.retain(|id| id != &app_id);
        }
    }
    
    Ok(())
}

async fn route_message(
    registry: &Arc<Mutex<BrokerRegistry>>,
    sender_id: &str,
    msg: IpcEnvelope,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match &msg.channel {
        EventBusChannel::Publish(ref payload) => {
            let sub_ids = {
                let reg = registry.lock().await;
                reg.subscribers.get(&payload.topic).cloned().unwrap_or_default()
            };
            
            let broadcast_msg = IpcEnvelope {
                sender_id: sender_id.to_string(),
                message_id: generate_msg_id(),
                timestamp: Utc::now(),
                channel: EventBusChannel::Broadcast(payload.clone()),
            };
            
            let reg = registry.lock().await;
            for sub_id in sub_ids {
                if sub_id != sender_id {
                    if let Some(conn) = reg.clients.get(&sub_id) {
                        let _ = conn.sender.send(broadcast_msg.clone());
                    }
                }
            }
        }
        EventBusChannel::Subscribe(ref payload) => {
            let mut reg = registry.lock().await;
            let subs = reg.subscribers.entry(payload.topic.clone()).or_default();
            if !subs.contains(&sender_id.to_string()) {
                subs.push(sender_id.to_string());
            }
        }
        EventBusChannel::Request(ref payload) => {
            // Handle system query for active clients
            if payload.service == "_system.list_clients" {
                let manifests: Vec<AppManifest> = {
                    let reg = registry.lock().await;
                    reg.clients.values().map(|conn| conn.manifest.clone()).collect()
                };
                
                let response = IpcEnvelope {
                    sender_id: "broker".to_string(),
                    message_id: generate_msg_id(),
                    timestamp: Utc::now(),
                    channel: EventBusChannel::Response(ResponsePayload {
                        request_id: msg.message_id.clone(),
                        result: Ok(serde_json::to_value(manifests).unwrap()),
                    }),
                };
                
                let reg = registry.lock().await;
                if let Some(conn) = reg.clients.get(sender_id) {
                    let _ = conn.sender.send(response);
                }
                return Ok(());
            }

            let provider_id = {
                let reg = registry.lock().await;
                reg.clients.values()
                    .find(|conn| conn.manifest.published_services.contains(&payload.service))
                    .map(|conn| conn.app_id.clone())
            };
            
            if let Some(ref provider) = provider_id {
                {
                    let mut reg = registry.lock().await;
                    reg.pending_requests.insert(msg.message_id.clone(), sender_id.to_string());
                }
                
                let reg = registry.lock().await;
                if let Some(conn) = reg.clients.get(provider) {
                    let _ = conn.sender.send(msg);
                }
            } else {
                let response = IpcEnvelope {
                    sender_id: "broker".to_string(),
                    message_id: generate_msg_id(),
                    timestamp: Utc::now(),
                    channel: EventBusChannel::Response(ResponsePayload {
                        request_id: msg.message_id.clone(),
                        result: Err(IpcError::ServiceNotFound(payload.service.clone())),
                    }),
                };
                let reg = registry.lock().await;
                if let Some(conn) = reg.clients.get(sender_id) {
                    let _ = conn.sender.send(response);
                }
            }
        }
        EventBusChannel::Response(ref payload) => {
            let requester_id = {
                let mut reg = registry.lock().await;
                reg.pending_requests.remove(&payload.request_id)
            };
            
            if let Some(ref requester) = requester_id {
                let reg = registry.lock().await;
                if let Some(conn) = reg.clients.get(requester) {
                    let _ = conn.sender.send(msg);
                }
            } else {
                warn!("Received response for unknown request_id '{}'", payload.request_id);
            }
        }
        EventBusChannel::Broadcast(_) => {
            warn!("Client {} tried to send a Broadcast envelope", sender_id);
        }
    }
    Ok(())
}

pub struct IpcClient {
    app_id: String,
    tx: mpsc::UnboundedSender<IpcEnvelope>,
    pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, IpcError>>>>>,
}

impl IpcClient {
    pub async fn connect(
        app_id: &str,
        manifest: AppManifest,
        pipe_name: &str,
    ) -> Result<(Self, mpsc::UnboundedReceiver<IpcEnvelope>), std::io::Error> {
        // Handle Windows pipe busy error (ERROR_PIPE_BUSY = 231)
        let client = loop {
            match ClientOptions::new().open(pipe_name) {
                Ok(c) => break c,
                Err(e) if e.raw_os_error() == Some(231) => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        };

        let (mut reader, mut writer) = tokio::io::split(client);

        let (tx, mut rx) = mpsc::unbounded_channel::<IpcEnvelope>();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<IpcEnvelope>();
        let pending_responses = Arc::new(Mutex::new(HashMap::<
            String,
            oneshot::Sender<Result<serde_json::Value, IpcError>>,
        >::new()));
        let pending_responses_clone = Arc::clone(&pending_responses);

        // Handshake: Send AppManifest
        let handshake = IpcEnvelope {
            sender_id: app_id.to_string(),
            message_id: generate_msg_id(),
            timestamp: Utc::now(),
            channel: EventBusChannel::Publish(EventPayload {
                topic: "CapabilityPublish".to_string(),
                data: serde_json::to_value(manifest).unwrap(),
            }),
        };
        let handshake_bytes = serde_json::to_vec(&handshake).unwrap();
        write_frame(&mut writer, &handshake_bytes).await?;

        // Spawn writer task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let serialized = match serde_json::to_vec(&msg) {
                    Ok(b) => b,
                    Err(e) => {
                        error!("[Client] Failed to serialize message: {}", e);
                        continue;
                    }
                };
                if let Err(e) = write_frame(&mut writer, &serialized).await {
                    error!("[Client] Error writing to pipe: {}", e);
                    break;
                }
            }
        });

        // Spawn reader task
        tokio::spawn(async move {
            loop {
                match read_frame(&mut reader).await {
                    Ok(bytes) => {
                        let envelope: IpcEnvelope = match serde_json::from_slice(&bytes) {
                            Ok(env) => env,
                            Err(e) => {
                                error!("[Client] Failed to deserialize envelope: {}", e);
                                continue;
                            }
                        };
                        
                        if let EventBusChannel::Response(resp) = &envelope.channel {
                            let mut map = pending_responses_clone.lock().await;
                            if let Some(tx) = map.remove(&resp.request_id) {
                                let _ = tx.send(resp.result.clone());
                            }
                        } else {
                            if incoming_tx.send(envelope).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        info!("[Client] Reader loop disconnected: {}", e);
                        break;
                    }
                }
            }
        });

        let client = Self {
            app_id: app_id.to_string(),
            tx,
            pending_responses,
        };

        Ok((client, incoming_rx))
    }

    pub fn publish(&self, topic: &str, data: serde_json::Value) -> Result<(), String> {
        let msg = IpcEnvelope {
            sender_id: self.app_id.clone(),
            message_id: generate_msg_id(),
            timestamp: Utc::now(),
            channel: EventBusChannel::Publish(EventPayload {
                topic: topic.to_string(),
                data,
            }),
        };
        self.tx.send(msg).map_err(|e| e.to_string())
    }

    pub fn subscribe(&self, topic: &str) -> Result<(), String> {
        let msg = IpcEnvelope {
            sender_id: self.app_id.clone(),
            message_id: generate_msg_id(),
            timestamp: Utc::now(),
            channel: EventBusChannel::Subscribe(SubscribePayload {
                topic: topic.to_string(),
            }),
        };
        self.tx.send(msg).map_err(|e| e.to_string())
    }

    pub async fn request(&self, service: &str, params: serde_json::Value) -> Result<serde_json::Value, IpcError> {
        let message_id = generate_msg_id();
        let msg = IpcEnvelope {
            sender_id: self.app_id.clone(),
            message_id: message_id.clone(),
            timestamp: Utc::now(),
            channel: EventBusChannel::Request(RequestPayload {
                service: service.to_string(),
                params,
            }),
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending_responses.lock().await;
            map.insert(message_id, tx);
        }

        self.tx.send(msg).map_err(|e| IpcError::Internal(e.to_string()))?;

        tokio::select! {
            res = rx => {
                res.map_err(|_| IpcError::Internal("Receiver channel closed".to_string()))?
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                Err(IpcError::Timeout)
            }
        }
    }

    pub fn respond(&self, request_id: &str, result: Result<serde_json::Value, IpcError>) -> Result<(), String> {
        let msg = IpcEnvelope {
            sender_id: self.app_id.clone(),
            message_id: generate_msg_id(),
            timestamp: Utc::now(),
            channel: EventBusChannel::Response(ResponsePayload {
                request_id: request_id.to_string(),
                result,
            }),
        };
        self.tx.send(msg).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use liem_common::AppManifest;

    #[tokio::test]
    async fn test_ipc_pub_sub_rpc() {
        let pipe_name = r"\\.\pipe\liem-ipc-test-pipe";
        let broker = IpcBroker::new(pipe_name);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        
        let broker_handle = tokio::spawn(async move {
            broker.start(shutdown_rx).await.unwrap();
        });

        // Let the broker open its named pipe listener
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let manifest1 = AppManifest {
            app_id: "client1".to_string(),
            name: "Client 1".to_string(),
            version: "1.0.0".to_string(),
            protocol_version: 1,
            capabilities: vec![],
            published_services: vec!["test.service".to_string()],
            event_subscriptions: vec![],
        };

        let manifest2 = AppManifest {
            app_id: "client2".to_string(),
            name: "Client 2".to_string(),
            version: "1.0.0".to_string(),
            protocol_version: 1,
            capabilities: vec![],
            published_services: vec![],
            event_subscriptions: vec!["test.topic".to_string()],
        };

        let (client1, mut rx1) = IpcClient::connect("client1", manifest1, pipe_name).await.unwrap();
        let (client2, mut rx2) = IpcClient::connect("client2", manifest2, pipe_name).await.unwrap();

        // client2 subscribes to "test.topic"
        client2.subscribe("test.topic").unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // client1 publishes to "test.topic"
        client1.publish("test.topic", serde_json::json!({"hello": "world"})).unwrap();

        // client2 should receive the broadcast
        let msg = tokio::select! {
            msg = rx2.recv() => msg.unwrap(),
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => panic!("Timeout waiting for broadcast"),
        };

        if let EventBusChannel::Broadcast(payload) = msg.channel {
            assert_eq!(payload.topic, "test.topic");
            assert_eq!(payload.data, serde_json::json!({"hello": "world"}));
        } else {
            panic!("Expected Broadcast message, got {:?}", msg.channel);
        }

        // Spawn client1 response service task
        tokio::spawn(async move {
            let req = rx1.recv().await.unwrap();
            if let EventBusChannel::Request(payload) = req.channel {
                assert_eq!(payload.service, "test.service");
                assert_eq!(payload.params, serde_json::json!({"param": 42}));
                client1.respond(&req.message_id, Ok(serde_json::json!({"result": "success"}))).unwrap();
            } else {
                panic!("Expected Request message");
            }
        });

        // client2 calls request on client1's service
        let res = client2.request("test.service", serde_json::json!({"param": 42})).await.unwrap();
        assert_eq!(res, serde_json::json!({"result": "success"}));

        // Clean up
        let _ = shutdown_tx.send(());
        let _ = broker_handle.await;
    }
}
