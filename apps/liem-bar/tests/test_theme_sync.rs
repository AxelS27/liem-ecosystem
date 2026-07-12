use tokio::sync::oneshot;
use liem_common::{AppManifest, EventBusChannel};
use liem_ipc::{IpcBroker, IpcClient};

#[tokio::test]
async fn test_ipc_theme_sync_broadcast() {
    // 1. Setup a unique named pipe for testing
    let pipe_name = r"\\.\pipe\liem-bar-theme-test";
    let broker = IpcBroker::new(pipe_name);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    
    let broker_handle = tokio::spawn(async move {
        let _ = broker.start(shutdown_rx).await;
    });

    // Wait for the broker to initialize the pipe server
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2. Connect client 1 (Subscriber)
    let manifest1 = AppManifest {
        app_id: "test.subscriber".to_string(),
        name: "Test Subscriber".to_string(),
        version: "0.1.0".to_string(),
        protocol_version: 1,
        capabilities: vec![],
        published_services: vec![],
        event_subscriptions: vec!["ThemeChange".to_string()],
    };

    let (client1, mut incoming_rx) = IpcClient::connect(
        "test.subscriber",
        manifest1,
        pipe_name,
    ).await.unwrap();

    let _ = client1.subscribe("ThemeChange");

    // 3. Connect client 2 (Publisher)
    let manifest2 = AppManifest {
        app_id: "test.publisher".to_string(),
        name: "Test Publisher".to_string(),
        version: "0.1.0".to_string(),
        protocol_version: 1,
        capabilities: vec![],
        published_services: vec![],
        event_subscriptions: vec![],
    };

    let (client2, _) = IpcClient::connect(
        "test.publisher",
        manifest2,
        pipe_name,
    ).await.unwrap();

    // 4. Publish ThemeChange event from Publisher
    let theme_data = serde_json::json!({
        "accent": "#ff5555",
        "transition": "slide"
    });
    let _ = client2.publish("ThemeChange", theme_data);

    // 5. Receive ThemeChange on Subscriber
    let msg = tokio::select! {
        Some(m) = incoming_rx.recv() => m,
        _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
            panic!("Timeout waiting for ThemeChange IPC event");
        }
    };

    // 6. Validate message envelope
    if let EventBusChannel::Broadcast(payload) = msg.channel {
        assert_eq!(payload.topic, "ThemeChange");
        assert_eq!(payload.data.get("accent").unwrap().as_str().unwrap(), "#ff5555");
    } else {
        panic!("Received unexpected message channel type");
    }

    // 7. Cleanup Broker
    let _ = shutdown_tx.send(());
    let _ = broker_handle.await;
}
