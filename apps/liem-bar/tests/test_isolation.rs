use std::sync::Arc;
use liem_sdk::Plugin;

use liem_bar::core::config::LayoutNode;
use liem_bar::core::module::{
    BarModule, Capability, ModuleMetadata, Permission, ServiceContext, WidgetDefinition,
};
use liem_bar::core::module_manager::{ModuleManager, ModuleRuntimeState};
use liem_bar::core::renderer::Renderer;

// Mock Renderer for testing
struct MockRenderer;
impl Renderer for MockRenderer {
    fn create_bar(&mut self, _: &str, _: liem_bar::core::config::BarPosition, _: u32, _: u32, _: u32, _: bool) -> Result<(), String> {
        Ok(())
    }
    fn render_layout_tree(&mut self, _: &LayoutNode) -> Result<(), String> {
        Ok(())
    }
    fn apply_theme(&mut self, _: &liem_bar::core::config::ThemeConfig) -> Result<(), String> {
        Ok(())
    }
    fn apply_css(&mut self, _: &std::collections::HashMap<String, liem_bar::core::theme::CssStyle>) -> Result<(), String> {
        Ok(())
    }
    fn set_visible(&mut self, _: bool) -> Result<(), String> {
        Ok(())
    }
    fn run(&mut self) -> Result<(), String> {
        Ok(())
    }
}

// 1. Panicking Module
struct PanickerModule;
impl Plugin for PanickerModule {
    fn on_enable(&self) -> Result<(), String> { Ok(()) }
    fn on_disable(&self) -> Result<(), String> { Ok(()) }
}
impl BarModule for PanickerModule {
    fn id(&self) -> &'static str { "test.panicker" }
    fn name(&self) -> &'static str { "Panicker" }
    fn widgets(&self) -> Vec<WidgetDefinition> { vec![] }
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: self.id().to_string(),
            name: self.name().to_string(),
            version: "0.1.0".to_string(),
            author: "Test".to_string(),
            api_version: 1,
            min_sdk_version: "0.1.0".to_string(),
            max_sdk_version: "0.2.0".to_string(),
            permissions: vec![Permission::None],
            capabilities: vec![Capability::Widget],
            dependencies: vec![],
        }
    }
    fn init_ui(&self, _: &str, _: &slint::Window) -> Result<(), String> {
        Ok(())
    }
    fn on_tick(&self, _: &ServiceContext) -> Result<(), String> {
        panic!("Intended testing panic inside tick!");
    }
}

// 2. Unresolved Dependency Module
struct DependentModule;
impl Plugin for DependentModule {
    fn on_enable(&self) -> Result<(), String> { Ok(()) }
    fn on_disable(&self) -> Result<(), String> { Ok(()) }
}
impl BarModule for DependentModule {
    fn id(&self) -> &'static str { "test.dependent" }
    fn name(&self) -> &'static str { "Dependent" }
    fn widgets(&self) -> Vec<WidgetDefinition> { vec![] }
    fn dependencies(&self) -> Vec<String> {
        vec!["missing.dependency.id".to_string()]
    }
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: self.id().to_string(),
            name: self.name().to_string(),
            version: "0.1.0".to_string(),
            author: "Test".to_string(),
            api_version: 1,
            min_sdk_version: "0.1.0".to_string(),
            max_sdk_version: "0.2.0".to_string(),
            permissions: vec![Permission::None],
            capabilities: vec![Capability::Widget],
            dependencies: self.dependencies(),
        }
    }
    fn init_ui(&self, _: &str, _: &slint::Window) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn test_crash_isolation_and_dependencies() {
    let mut manager = ModuleManager::new();
    
    let panicker = Arc::new(PanickerModule);
    let dependent = Arc::new(DependentModule);
    
    manager.register_module(panicker.clone());
    manager.register_module(dependent.clone());

    // Initially all should be in Created state
    assert_eq!(manager.get_state("test.panicker"), Some(ModuleRuntimeState::Created));
    assert_eq!(manager.get_state("test.dependent"), Some(ModuleRuntimeState::Created));

    // Resolve dependencies
    manager.resolve_dependencies();

    // Panicker has no dependencies -> should transition to Running/waiting to run
    // Dependent has missing dependency -> transitions to WaitingDependency
    assert_eq!(manager.get_state("test.panicker"), Some(ModuleRuntimeState::Running));
    assert_eq!(manager.get_state("test.dependent"), Some(ModuleRuntimeState::WaitingDependency));

    // Perform tick
    let ctx = ServiceContext {
        battery: None,
        network: None,
        audio: None,
        monitor: None,
        system: None,
    };
    let mut renderer = MockRenderer;
    let layout = LayoutNode::Spacer;

    // Tick the manager - Panicker should panic, but tick should catch it and continue!
    manager.tick(&ctx);

    // Panicker should now be in Error state
    assert_eq!(manager.get_state("test.panicker"), Some(ModuleRuntimeState::Error));
    // Dependent remains in WaitingDependency
    assert_eq!(manager.get_state("test.dependent"), Some(ModuleRuntimeState::WaitingDependency));
}
