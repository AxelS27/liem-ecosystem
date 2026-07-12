use std::sync::Arc;
use serde_json::Value;
use liem_sdk::Plugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    None,
    Internet,
    MediaSession,
    WallpaperAccess,
    SystemTray,
    ActiveWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Widget,
    NotificationProvider,
    MediaController,
    SystemMonitor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceCost {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionSource {
    Interval(u32),
    SystemEvent(String),
    IpcTopic(String),
}

pub struct ModuleMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub api_version: u32,
    pub min_sdk_version: String,
    pub max_sdk_version: String,
    pub permissions: Vec<Permission>,
    pub capabilities: Vec<Capability>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WidgetDefinition {
    pub widget_id: &'static str,
    pub name: &'static str,
    pub flex_weight: f32,
}

pub trait BatteryService: Send + Sync {
    fn is_on_ac(&self) -> bool { true }
    fn percent(&self) -> u8 { 100 }
    fn is_saver_active(&self) -> bool { false }
}
pub trait NetworkService: Send + Sync {}
pub trait AudioService: Send + Sync {}
pub trait MonitorService: Send + Sync {}
pub trait SystemService: Send + Sync {}

pub struct ServiceContext {
    pub battery: Option<Arc<dyn BatteryService>>,
    pub network: Option<Arc<dyn NetworkService>>,
    pub audio: Option<Arc<dyn AudioService>>,
    pub monitor: Option<Arc<dyn MonitorService>>,
    pub system: Option<Arc<dyn SystemService>>,
}

pub trait BarModule: Plugin {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn resource_cost(&self) -> ResourceCost { ResourceCost::Low }
    fn permissions(&self) -> Vec<Permission> { vec![Permission::None] }
    fn capabilities(&self) -> Vec<Capability> { vec![Capability::Widget] }
    fn widgets(&self) -> Vec<WidgetDefinition>;
    fn dependencies(&self) -> Vec<String> { vec![] }
    fn subscriptions(&self) -> Vec<SubscriptionSource> { vec![] }
    fn metadata(&self) -> ModuleMetadata;

    fn on_config_changed(&self, _settings: &Value) -> Result<(), String> { Ok(()) }

    fn init_ui(&self, widget_id: &str, ui_handle: &slint::Window) -> Result<(), String>;
    fn on_tick(&self, _ctx: &ServiceContext) -> Result<(), String> { Ok(()) }

    fn on_click(&self, _widget_id: &str, _x: i32, _y: i32) -> Result<(), String> { Ok(()) }
    fn on_scroll(&self, _widget_id: &str, _delta: f32) -> Result<(), String> { Ok(()) }
    fn on_hover(&self, _widget_id: &str, _entered: bool) -> Result<(), String> { Ok(()) }
    fn on_destroy(&self) -> Result<(), String> { Ok(()) }
}
