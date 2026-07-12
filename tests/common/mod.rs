use tokio::sync::oneshot;
use liem_ipc::{IpcBroker, IpcClient};
use liem_common::AppManifest;

pub struct TestEnv {
    pub pipe_name: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
    _broker_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TestEnv {
    pub async fn start_new(test_id: &str) -> Self {
        // Generate a unique Windows Named Pipe name for the test run to avoid port/pipe collisions.
        let pipe_name = format!(r"\\.\pipe\liem-test-pipe-{}", test_id);
        let broker = IpcBroker::new(&pipe_name);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        
        let broker_handle = tokio::spawn(async move {
            if let Err(e) = broker.start(shutdown_rx).await {
                eprintln!("[TestBroker] Broker failed: {}", e);
            }
        });

        // Give the broker a small amount of time to initialize the pipe server and listen.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        Self {
            pipe_name,
            shutdown_tx: Some(shutdown_tx),
            _broker_handle: Some(broker_handle),
        }
    }

    pub async fn connect_client(
        &self,
        app_id: &str,
        manifest: AppManifest,
    ) -> (IpcClient, tokio::sync::mpsc::UnboundedReceiver<liem_common::IpcEnvelope>) {
        IpcClient::connect(app_id, manifest, &self.pipe_name)
            .await
            .unwrap()
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
