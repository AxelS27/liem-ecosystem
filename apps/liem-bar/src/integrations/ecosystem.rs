#[cfg(feature = "ipc")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "ipc")]
use liem_common::AppManifest;
#[cfg(feature = "ipc")]
use liem_ipc::IpcClient;
#[cfg(feature = "ipc")]
use crate::core::module_manager::ModuleManager;

#[cfg(feature = "ipc")]
pub fn start_ecosystem_client(
    manager: Arc<Mutex<ModuleManager>>,
) {
    let manager_clone = manager.clone();
    tokio::spawn(async move {
        let manifest = AppManifest {
            app_id: "org.liem.bar".to_string(),
            name: "Liem Bar".to_string(),
            version: "0.1.0".to_string(),
            protocol_version: 1,
            capabilities: vec!["theme.subscriber".to_string(), "remote.receiver".to_string()],
            published_services: vec![],
            event_subscriptions: vec![
                "ThemeChange".to_string(),
                "RemoteModuleRegister".to_string(),
                "RemoteModuleHeartbeat".to_string(),
                "RemoteModuleData".to_string(),
            ],
        };

        // Try connecting to the Ecosystem Event Bus broker named pipe
        let (client, mut incoming_rx) = match IpcClient::connect(
            "org.liem.bar",
            manifest,
            r"\\.\pipe\liem-event-bus",
        ).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[Ecosystem IPC] Failed to connect to Ecosystem Event Bus: {}. Sync disabled.", e);
                return;
            }
        };

        println!("[Ecosystem IPC] Connected to Ecosystem Event Bus!");
        let _ = client.subscribe("ThemeChange");
        let _ = client.subscribe("RemoteModuleRegister");
        let _ = client.subscribe("RemoteModuleHeartbeat");
        let _ = client.subscribe("RemoteModuleData");

        while let Some(msg) = incoming_rx.recv().await {
            if let liem_common::EventBusChannel::Broadcast(payload) = msg.channel {
                match payload.topic.as_str() {
                    "ThemeChange" => {
                        println!("[Ecosystem IPC] Received ThemeChange event: {:?}", payload.data);
                        if let Some(accent) = payload.data.get("accent").and_then(|v| v.as_str()) {
                            let accent_color = accent.to_string();
                            let active_weaks = crate::core::renderer::get_active_windows();
                            for weak in active_weaks {
                                let accent_clone = accent_color.clone();
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(window) = weak.upgrade() {
                                        window.set_border_color(slint::Brush::SolidColor(
                                            crate::core::theme::parse_color(&accent_clone)
                                        ));
                                    }
                                });
                            }
                        }
                    }
                    "RemoteModuleRegister" => {
                        println!("[Ecosystem IPC] Received RemoteModuleRegister: {:?}", payload.data);
                        if let (Some(id), Some(name)) = (
                            payload.data.get("id").and_then(|v| v.as_str()),
                            payload.data.get("name").and_then(|v| v.as_str()),
                        ) {
                            let widget_ids: Vec<String> = payload.data.get("widgets")
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.iter().filter_map(|w| w.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default();
                            
                            let timeout_secs = payload.data.get("timeout_secs")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(10);

                            let mut mgr = manager_clone.lock().unwrap();
                            mgr.register_remote_module(id, name, widget_ids, timeout_secs);
                        }
                    }
                    "RemoteModuleHeartbeat" => {
                        if let Some(id) = payload.data.get("id").and_then(|v| v.as_str()) {
                            let mut mgr = manager_clone.lock().unwrap();
                            mgr.update_remote_heartbeat(id);
                        }
                    }
                    "RemoteModuleData" => {
                        if let (Some(widget_id), Some(data)) = (
                            payload.data.get("widget_id").and_then(|v| v.as_str()),
                            payload.data.get("data").and_then(|v| v.as_str()),
                        ) {
                            let mut mgr = manager_clone.lock().unwrap();
                            mgr.update_remote_data(widget_id, data);
                        }
                    }
                    _ => {}
                }
            }
        }
    });
}

#[cfg(not(feature = "ipc"))]
pub fn start_ecosystem_client(_manager: std::sync::Arc<std::sync::Mutex<crate::core::module_manager::ModuleManager>>) {}
