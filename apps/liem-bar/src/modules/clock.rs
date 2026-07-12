use std::sync::Mutex;
use chrono::Local;
use liem_sdk::Plugin;

use crate::core::module::{
    BarModule, Capability, ModuleMetadata, Permission, ServiceContext, SubscriptionSource,
    WidgetDefinition,
};

pub struct ClockModule {
    window_weak: Mutex<Option<slint::Weak<crate::core::renderer::MainWindow>>>,
}

impl ClockModule {
    pub fn new() -> Self {
        Self {
            window_weak: Mutex::new(None),
        }
    }
}

impl Default for ClockModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ClockModule {
    fn on_enable(&self) -> Result<(), String> {
        Ok(())
    }

    fn on_disable(&self) -> Result<(), String> {
        Ok(())
    }
}

impl BarModule for ClockModule {
    fn id(&self) -> &'static str {
        "org.liem.clock"
    }

    fn name(&self) -> &'static str {
        "Clock Module"
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
                widget_id: "clock.time",
                name: "Current Time",
                flex_weight: 1.0,
            },
            WidgetDefinition {
                widget_id: "clock.date",
                name: "Current Date",
                flex_weight: 1.0,
            },
        ]
    }

    fn subscriptions(&self) -> Vec<SubscriptionSource> {
        vec![SubscriptionSource::Interval(1000)]
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
                main_window.set_clock_time(now.format("%H:%M:%S").to_string().into());
                main_window.set_clock_date(now.format("%Y-%m-%d").to_string().into());
            }
        }
        Ok(())
    }
}
