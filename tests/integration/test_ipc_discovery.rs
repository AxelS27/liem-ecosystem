use liem_common::AppManifest;
use liem_common::version::{negotiate_version, negotiate_capabilities};
use liem_integration_tests::TestEnv;

#[tokio::test]
async fn test_service_discovery_lifecycle() {
    // 0. Setup: start test environment broker
    let env = TestEnv::start_new("discovery-lifecycle").await;

    // Client A (Wallpaper app) manifest
    let manifest_a = AppManifest {
        app_id: "org.liem.wallpaper".to_string(),
        name: "Liem Wallpaper".to_string(),
        version: "1.2.0".to_string(),
        protocol_version: 1,
        capabilities: vec!["theme.sync".to_string(), "wallpaper.changed".to_string()],
        published_services: vec![],
        event_subscriptions: vec![],
    };

    // Client B (Bar app) manifest
    let manifest_b = AppManifest {
        app_id: "org.liem.bar".to_string(),
        name: "Liem Bar".to_string(),
        version: "1.5.1".to_string(),
        protocol_version: 1,
        capabilities: vec!["theme.sync".to_string(), "bar.visible".to_string()],
        published_services: vec![],
        event_subscriptions: vec!["ThemeChange".to_string()],
    };

    // 5. Connect: Client A establishes pipe connection with broker
    let (client_a, _rx_a) = env.connect_client("org.liem.wallpaper", manifest_a).await;

    // 5. Connect: Client B establishes pipe connection with broker
    let (client_b, mut rx_b) = env.connect_client("org.liem.bar", manifest_b.clone()).await;

    // 1. Discover: Client B requests registered client list from the broker
    let res = client_b.request("_system.list_clients", serde_json::json!({})).await.unwrap();
    let manifests: Vec<AppManifest> = serde_json::from_value(res).unwrap();
    
    // Find Client A (Liem Wallpaper) in the list
    let peer_manifest = manifests.iter()
        .find(|m| m.app_id == "org.liem.wallpaper")
        .expect("Client A should be discovered in the active clients list");

    // 2. Read Manifest: Verify peer details match manifest expectation
    assert_eq!(peer_manifest.name, "Liem Wallpaper");
    assert_eq!(peer_manifest.version, "1.2.0");

    // 3. Negotiate Version: Verify version compatibility using SemVer
    let version_negotiation = negotiate_version(&peer_manifest.version, "1.0.0");
    assert!(version_negotiation.is_ok(), "Major versions match, should be compatible");

    // 4. Negotiate Capabilities: Determine matching capabilities intersection
    let shared_caps = negotiate_capabilities(&peer_manifest, &manifest_b);
    assert_eq!(shared_caps, vec!["theme.sync".to_string()]);

    // 6. Subscribe: Client B registers event subscription interest for "ThemeChange"
    client_b.subscribe("ThemeChange").unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // 7. Monitor: Client A publishes ThemeChange event, broker broadcasts to Client B
    client_a.publish("ThemeChange", serde_json::json!({"accent": "blue"})).unwrap();

    let event = tokio::select! {
        msg = rx_b.recv() => msg.unwrap(),
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => panic!("Timeout waiting for ThemeChange event"),
    };

    if let liem_common::EventBusChannel::Broadcast(payload) = event.channel {
        assert_eq!(payload.topic, "ThemeChange");
        assert_eq!(payload.data, serde_json::json!({"accent": "blue"}));
    } else {
        panic!("Expected Broadcast event on ThemeChange");
    }

    // 8. Disconnect: Drop clients (graceful cleanup happens via Drop implementation)
}
