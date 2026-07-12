use std::time::Duration;
use std::sync::{Arc, Mutex};
use liem_sdk::{ExtensionManifest, SecurityTier, validate_extension_trust};
use liem_bar::core::module_manager::{ModuleManager, ModuleRuntimeState};
use liem_bar::core::module::{BarModule, ServiceContext, ModuleMetadata, ResourceCost, Permission, Capability, WidgetDefinition};

#[test]
fn test_security_tier_developer_mode() {
    let manifest = ExtensionManifest {
        id: "com.test.widget".to_string(),
        name: "Test Widget".to_string(),
        version: "1.0.0".to_string(),
        author: "Tester".to_string(),
        entry_point: "test.dll".to_string(),
        security_tier: SecurityTier::Development,
        enabled: true,
    };

    // 1. Rejects development tier if developer_mode is false
    let res = validate_extension_trust(&manifest, false);
    assert!(res.is_err());
    assert!(res.err().unwrap().contains("Developer mode must be enabled"));

    // 2. Accepts if developer_mode is true
    let res = validate_extension_trust(&manifest, true);
    assert!(res.is_ok());

    // 3. Community tier accepted regardless of developer_mode
    let community = ExtensionManifest {
        security_tier: SecurityTier::Community,
        ..manifest.clone()
    };
    assert!(validate_extension_trust(&community, false).is_ok());
}

#[test]
fn test_remote_module_registration_and_heartbeat() {
    let mut manager = ModuleManager::new();
    
    // Register remote module with 1 second timeout
    manager.register_remote_module(
        "remote.test",
        "Test Remote",
        vec!["widget.cpu".to_string()],
        1,
    );

    // Verify registered and running
    assert_eq!(manager.get_state("remote.test"), Some(ModuleRuntimeState::Running));

    // Update remote data
    manager.update_remote_data("widget.cpu", "45%");
    assert_eq!(manager.remote_widget_data.get("widget.cpu").map(|s| s.as_str()), Some("45%"));

    // Check timeouts immediately (should not timeout)
    manager.check_remote_timeouts();
    assert_eq!(manager.get_state("remote.test"), Some(ModuleRuntimeState::Running));

    // Wait 1.5 seconds for heartbeat timeout
    std::thread::sleep(Duration::from_millis(1500));
    manager.check_remote_timeouts();

    // Verify automatically unregistered
    assert_eq!(manager.get_state("remote.test"), None);
    assert!(manager.remote_widget_data.get("widget.cpu").is_none());
}

#[test]
fn test_remote_module_heartbeat_renewal() {
    let mut manager = ModuleManager::new();
    
    // Register with 1 second timeout
    manager.register_remote_module(
        "remote.renew",
        "Renew Remote",
        vec!["widget.ram".to_string()],
        1,
    );

    // Wait 600ms, send heartbeat
    std::thread::sleep(Duration::from_millis(600));
    manager.update_remote_heartbeat("remote.renew");

    // Wait another 600ms (total 1.2s since registration, but only 600ms since heartbeat)
    std::thread::sleep(Duration::from_millis(600));
    manager.check_remote_timeouts();

    // Verify still running
    assert_eq!(manager.get_state("remote.renew"), Some(ModuleRuntimeState::Running));

    // Wait 1.2s without heartbeat
    std::thread::sleep(Duration::from_millis(1200));
    manager.check_remote_timeouts();

    // Verify now timed out
    assert_eq!(manager.get_state("remote.renew"), None);
}

// --- Resource Budgeting Tests ---

struct MockBattery(bool);
impl liem_bar::core::module::BatteryService for MockBattery {
    fn is_saver_active(&self) -> bool {
        self.0
    }
}

struct BudgetModule {
    tick_count: Mutex<u32>,
}
impl liem_sdk::Plugin for BudgetModule {
    fn on_enable(&self) -> Result<(), String> { Ok(()) }
    fn on_disable(&self) -> Result<(), String> { Ok(()) }
}
impl BarModule for BudgetModule {
    fn id(&self) -> &'static str { "test.budget" }
    fn name(&self) -> &'static str { "Budget Test" }
    fn resource_cost(&self) -> ResourceCost { ResourceCost::High }
    fn widgets(&self) -> Vec<WidgetDefinition> { vec![] }
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: "test.budget".to_string(),
            name: "Budget Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Tester".to_string(),
            api_version: 1,
            min_sdk_version: "0.1.0".to_string(),
            max_sdk_version: "2.0.0".to_string(),
            permissions: vec![],
            capabilities: vec![],
            dependencies: vec![],
        }
    }
    fn init_ui(&self, _widget_id: &str, _ui_handle: &slint::Window) -> Result<(), String> { Ok(()) }
    fn on_tick(&self, _ctx: &ServiceContext) -> Result<(), String> {
        let mut guard = self.tick_count.lock().unwrap();
        *guard += 1;
        Ok(())
    }
}

#[test]
fn test_resource_budgeting_throttling() {
    let mut manager = ModuleManager::new();
    let module = Arc::new(BudgetModule { tick_count: Mutex::new(0) });
    manager.register_module(module.clone());
    manager.resolve_dependencies();
    
    let ctx = ServiceContext {
        battery: Some(Arc::new(MockBattery(true))),
        network: None,
        audio: None,
        monitor: None,
        system: None,
    };
    
    for _ in 1..=9 {
        manager.tick(&ctx);
    }
    assert_eq!(*module.tick_count.lock().unwrap(), 0);
    
    manager.tick(&ctx);
    assert_eq!(*module.tick_count.lock().unwrap(), 1);
}
