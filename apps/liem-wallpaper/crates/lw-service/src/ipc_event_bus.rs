use std::sync::{Arc, Mutex};
use tracing::{info, error};
use liem_common::AppManifest;
use liem_ipc::{IpcBroker, IpcClient};

pub async fn start_ecosystem_ipc_loop(config: Arc<Mutex<lw_core::Config>>) {
    // 1. Attempt to spawn the ecosystem broker server
    // (If it fails because the pipe name is already in use, it means another Liem app
    // is already running the broker. That is completely normal and expected!)
    let broker = IpcBroker::new(r"\\.\pipe\liem-event-bus");
    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    
    tokio::spawn(async move {
        if let Err(_e) = broker.start(shutdown_rx).await {
            info!("[Ecosystem IPC] Another Liem application is hosting the Event Bus broker. Connecting as client...");
        } else {
            info!("[Ecosystem IPC] Hosted the Ecosystem Event Bus broker!");
        }
    });

    // Sleep briefly to let the broker spin up if we are hosting it
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // 2. Connect to the Ecosystem Event Bus broker as a client
    let version = env!("CARGO_PKG_VERSION");
    let manifest = AppManifest {
        app_id: "org.liem.wallpaper".to_string(),
        name: "Liem Wallpaper".to_string(),
        version: version.to_string(),
        protocol_version: 1,
        capabilities: vec!["theme.sync".to_string()],
        published_services: vec![],
        event_subscriptions: vec!["ThemeChange".to_string(), "_system.client_connected".to_string()],
    };

    let (client, mut incoming_rx) = match IpcClient::connect(
        "org.liem.wallpaper",
        manifest,
        r"\\.\pipe\liem-event-bus"
    ).await {
        Ok(c) => c,
        Err(e) => {
            error!("[Ecosystem IPC] Failed to connect to Ecosystem Event Bus: {}. Integration features disabled.", e);
            return;
        }
    };

    info!("[Ecosystem IPC] Connected to Ecosystem Event Bus!");

    // Subscribe to topics
    let _ = client.subscribe("ThemeChange");
    let _ = client.subscribe("_system.client_connected");

    // 3. Receive incoming messages
    tokio::spawn(async move {
        while let Some(msg) = incoming_rx.recv().await {
            match msg.channel {
                liem_common::EventBusChannel::Broadcast(payload) => {
                    if payload.topic == "ThemeChange" {
                        info!("[Ecosystem IPC] Received ThemeChange event: {:?}", payload.data);
                        if let Some(accent) = payload.data.get("accent").and_then(|v| v.as_str()) {
                            if let Some(transition) = payload.data.get("transition").and_then(|v| v.as_str()) {
                                // 1. Update active transition configuration
                                {
                                    let mut cfg = config.lock().unwrap();
                                    cfg.transition_default.effect_type = transition.to_string();
                                }
                                // 2. Trigger visual update in the Slint HUD
                                let hud_opt = crate::ui::HUD_HANDLE.lock().unwrap().clone();
                                if let Some(hud_weak) = hud_opt {
                                    let accent_clone = accent.to_string();
                                    let transition_clone = transition.to_string();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(hud) = hud_weak.upgrade() {
                                            hud.set_accent_color(crate::ui::parse_hex_color(&accent_clone));
                                            hud.set_transition_style(transition_clone.into());
                                        }
                                    });
                                }
                                info!("[Theme] Applied accent color: {} and transition style: {}", accent, transition);
                            }
                        }
                    } else if payload.topic == "_system.client_connected" {
                        info!("[Ecosystem IPC] Received client connected notification: {:?}", payload.data);
                    }
                }
                _ => {}
            }
        }
    });
}
