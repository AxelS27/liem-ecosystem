use std::sync::Mutex;
use chrono::Local;
use liem_sdk::Plugin;

use crate::core::module::{
    BarModule, Capability, ModuleMetadata, Permission, ServiceContext, SubscriptionSource,
    WidgetDefinition,
};

pub struct CalendarModule {
    window_weak: Mutex<Option<slint::Weak<crate::core::renderer::MainWindow>>>,
}

impl CalendarModule {
    pub fn new() -> Self {
        Self {
            window_weak: Mutex::new(None),
        }
    }
}

impl Default for CalendarModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for CalendarModule {
    fn on_enable(&self) -> Result<(), String> {
        Ok(())
    }

    fn on_disable(&self) -> Result<(), String> {
        Ok(())
    }
}

impl BarModule for CalendarModule {
    fn id(&self) -> &'static str {
        "org.liem.calendar"
    }

    fn name(&self) -> &'static str {
        "Calendar Module"
    }

    fn permissions(&self) -> Vec<Permission> {
        vec![Permission::None]
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Widget]
    }

    fn widgets(&self) -> Vec<WidgetDefinition> {
        vec![
            WidgetDefinition {
                widget_id: "calendar.grid",
                name: "Calendar Grid",
                flex_weight: 1.0,
            },
        ]
    }

    fn subscriptions(&self) -> Vec<SubscriptionSource> {
        vec![SubscriptionSource::Interval(3600000)]
    }

    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: self.id().to_string(),
            name: self.name().to_string(),
            version: "0.1.0".to_string(),
            author: "Liem Ecosystem".to_string(),
            api_version: 1,
            min_sdk_version: "0.1.0".to_string(),
            max_sdk_version: "0.2.0".to_string(),
            permissions: self.permissions(),
            capabilities: self.capabilities(),
            dependencies: vec![],
        }
    }

    fn init_ui(&self, _widget_id: &str, ui_handle: &slint::Window) -> Result<(), String> {
        if let Some(weak_window) = crate::core::renderer::get_window_component(ui_handle) {
            let mut weak_guard = self.window_weak.lock().unwrap();
            *weak_guard = Some(weak_window);
        }
        Ok(())
    }

    fn on_tick(&self, _ctx: &ServiceContext) -> Result<(), String> {
        let weak_guard = self.window_weak.lock().unwrap();
        if let Some(ref weak) = *weak_guard {
            if let Some(main_window) = weak.upgrade() {
                let now = Local::now();
                main_window.set_calendar_text(now.format("%B %Y").to_string().into());
            }
        }
        Ok(())
    }
}
