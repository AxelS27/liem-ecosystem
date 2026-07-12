use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::core::config::LayoutNode;
use crate::core::module::{BarModule, ServiceContext, SubscriptionSource};
use crate::core::renderer::Renderer;

pub struct ReactiveScheduler {
    modules: Vec<Arc<dyn BarModule>>,
    layout: LayoutNode,
    last_ticks: Vec<(String, Instant, u32)>,
}

impl ReactiveScheduler {
    pub fn new(modules: Vec<Arc<dyn BarModule>>, layout: LayoutNode) -> Self {
        let mut last_ticks = Vec::new();
        for m in &modules {
            for sub in m.subscriptions() {
                if let SubscriptionSource::Interval(ms) = sub {
                    // Set initial ticks to the past so they trigger immediately
                    let past_instant = Instant::now() - Duration::from_millis(ms as u64);
                    last_ticks.push((m.id().to_string(), past_instant, ms));
                }
            }
        }
        Self {
            modules,
            layout,
            last_ticks,
        }
    }

    /// Trigger periodic checks on all active module interval subscriptions.
    pub fn tick<R: Renderer>(&mut self, renderer: &mut R) {
        let now = Instant::now();
        let ctx = ServiceContext {
            battery: None,
            network: None,
            audio: None,
            monitor: None,
            system: None,
        };

        let mut dynamic_update_required = false;

        for m in &self.modules {
            let module_id = m.id();
            let mut trigger_tick = false;

            for (id, last_tick, interval) in &mut self.last_ticks {
                if id == module_id && now.duration_since(*last_tick).as_millis() >= *interval as u128 {
                    *last_tick = now;
                    trigger_tick = true;
                }
            }

            if trigger_tick {
                if let Err(e) = m.on_tick(&ctx) {
                    eprintln!("Error ticking module '{}': {}", module_id, e);
                }
                dynamic_update_required = true;
            }
        }

        // Batch layout tree updates only when modules have ticked
        if dynamic_update_required {
            if let Err(e) = renderer.render_layout_tree(&self.layout) {
                eprintln!("Failed to render layout tree: {}", e);
            }
        }
    }
}
