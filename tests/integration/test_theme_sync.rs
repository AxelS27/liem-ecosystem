use liem_common::AppManifest;
use liem_integration_tests::TestEnv;

#[tokio::test]
async fn test_theme_propagation_lifecycle() {
    // 0. Setup: start test environment broker
    let env = TestEnv::start_new("theme-sync").await;

    // Client A (Wallpaper app) manifest
    let manifest_a = AppManifest {
        app_id: "org.liem.wallpaper".to_string(),
        name: "Liem Wallpaper".to_string(),
        version: "1.0.0".to_string(),
        protocol_version: 1,
        capabilities: vec!["theme.sync".to_string()],
        published_services: vec![],
        event_subscriptions: vec!["ThemeChange".to_string()],
    };

    // Client B (Settings app / CLI) manifest
    let manifest_b = AppManifest {
        app_id: "org.liem.settings".to_string(),
        name: "Liem Settings".to_string(),
        version: "1.0.0".to_string(),
        protocol_version: 1,
        capabilities: vec![],
        published_services: vec![],
        event_subscriptions: vec![],
    };

    // Connect Client A and Client B
    let (client_a, mut rx_a) = env.connect_client("org.liem.wallpaper", manifest_a).await;
    let (client_b, _rx_b) = env.connect_client("org.liem.settings", manifest_b).await;

    // Client A subscribes to "ThemeChange"
    client_a.subscribe("ThemeChange").unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Client B publishes a ThemeChange event
    let test_payload = serde_json::json!({
        "accent": "#FF0055",
        "transition": "glitch"
    });
    client_b.publish("ThemeChange", test_payload.clone()).unwrap();

    // Verify Client A receives the event
    let event = tokio::select! {
        msg = rx_a.recv() => msg.unwrap(),
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => panic!("Timeout waiting for ThemeChange event"),
    };

    if let liem_common::EventBusChannel::Broadcast(payload) = event.channel {
        assert_eq!(payload.topic, "ThemeChange");
        assert_eq!(payload.data, test_payload);
        
        // Simulating the application applying the change to its configuration
        let accent = payload.data.get("accent").unwrap().as_str().unwrap();
        let transition = payload.data.get("transition").unwrap().as_str().unwrap();
        
        assert_eq!(accent, "#FF0055");
        assert_eq!(transition, "glitch");
    } else {
        panic!("Expected Broadcast event");
    }
}
