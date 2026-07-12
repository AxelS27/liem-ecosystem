use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::{Duration, Instant};

use liem_sdk::Plugin;
use crate::core::module::{BarModule, ServiceContext, ModuleMetadata, WidgetDefinition, ResourceCost};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleRuntimeState {
    Created,
    Initializing,
    WaitingDependency,
    Running,
    Suspended,
    Stopping,
    Stopped,
    Error,
}

pub struct ModuleEntry {
    pub module: Arc<dyn BarModule>,
    pub state: ModuleRuntimeState,
    pub error_count: u32,
    pub next_retry: Option<Instant>,
    pub cooldown_duration: Duration,
    
    // Heartbeat tracking for remote modules
    pub last_heartbeat: Option<Instant>,
    pub heartbeat_timeout: Option<Duration>,
}

pub struct ModuleManager {
    entries: Vec<ModuleEntry>,
    #[cfg(feature = "community")]
    pub plugins: Vec<Box<dyn Plugin>>,
    pub remote_widget_data: HashMap<String, String>,
    pub tick_count: u64,
}

impl ModuleManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            #[cfg(feature = "community")]
            plugins: Vec::new(),
            remote_widget_data: HashMap::new(),
            tick_count: 0,
        }
    }

    /// Register a new module to be managed.
    pub fn register_module(&mut self, module: Arc<dyn BarModule>) {
        self.entries.push(ModuleEntry {
            module,
            state: ModuleRuntimeState::Created,
            error_count: 0,
            next_retry: None,
            cooldown_duration: Duration::from_secs(2),
            last_heartbeat: None,
            heartbeat_timeout: None,
        });
    }

    pub fn get_modules(&self) -> Vec<Arc<dyn BarModule>> {
        self.entries.iter().map(|e| e.module.clone()).collect()
    }

    pub fn get_state(&self, module_id: &str) -> Option<ModuleRuntimeState> {
        self.entries.iter().find(|e| e.module.id() == module_id).map(|e| e.state)
    }

    /// Resolve dependencies across all registered modules and update state transitions.
    pub fn resolve_dependencies(&mut self) {
        let running_ids: Vec<String> = self.entries.iter()
            .filter(|e| e.state == ModuleRuntimeState::Running)
            .map(|e| e.module.id().to_string())
            .collect();

        for entry in &mut self.entries {
            if entry.state == ModuleRuntimeState::Running || entry.state == ModuleRuntimeState::WaitingDependency || entry.state == ModuleRuntimeState::Created {
                let mut all_met = true;
                for dep in entry.module.dependencies() {
                    if !running_ids.contains(&dep) {
                        all_met = false;
                        break;
                    }
                }

                if all_met {
                    if entry.state == ModuleRuntimeState::WaitingDependency || entry.state == ModuleRuntimeState::Created {
                        entry.state = ModuleRuntimeState::Running;
                    }
                } else {
                    entry.state = ModuleRuntimeState::WaitingDependency;
                }
            }
        }
    }

    /// Initialize UI on all modules, wrapped with panic isolation guards.
    pub fn init_ui(&mut self, widget_id: &str, ui_handle: &slint::Window) -> Result<(), String> {
        for entry in &mut self.entries {
            if entry.state == ModuleRuntimeState::Error {
                continue;
            }

            let module = entry.module.clone();
            let res = catch_unwind(AssertUnwindSafe(|| {
                module.init_ui(widget_id, ui_handle)
            }));

            match res {
                Ok(Ok(())) => {
                    entry.state = ModuleRuntimeState::Running;
                }
                Ok(Err(e)) => {
                    eprintln!("Module '{}' failed init_ui: {}", module.id(), e);
                    Self::quarantine_module(entry, Some(e));
                }
                Err(panic_err) => {
                    let panic_msg = parse_panic_message(&panic_err);
                    eprintln!("Module '{}' panicked during init_ui: {}", module.id(), panic_msg);
                    Self::quarantine_module(entry, Some(panic_msg));
                }
            }
        }

        self.resolve_dependencies();
        Ok(())
    }

    /// Centrally tick running modules under panic isolation, routing recovery cooldown timers.
    /// Returns true if any module succeeded, indicating a rendering update is required.
    pub fn tick(&mut self, ctx: &ServiceContext) -> bool {
        self.tick_count = self.tick_count.wrapping_add(1);
        let now = Instant::now();

        // 1. Check for remote module heartbeat timeouts
        self.check_remote_timeouts();

        // 2. Recover/retry modules currently quarantined in Error state
        for entry in &mut self.entries {
            if entry.state == ModuleRuntimeState::Error {
                if let Some(retry_time) = entry.next_retry {
                    if now >= retry_time {
                        println!("Retrying module '{}' recovery cooldown...", entry.module.id());
                        entry.state = ModuleRuntimeState::Created;
                    }
                }
            }
        }

        // 3. Resolve dependencies in case states have changed
        self.resolve_dependencies();

        let mut dirty = false;
        let is_saver = ctx.battery.as_ref().map(|b| b.is_saver_active()).unwrap_or(false);

        // 4. Tick all active running modules
        for entry in &mut self.entries {
            if entry.state != ModuleRuntimeState::Running {
                continue;
            }

            if is_saver {
                match entry.module.resource_cost() {
                    ResourceCost::High => {
                        if self.tick_count % 10 != 0 {
                            continue;
                        }
                    }
                    ResourceCost::Medium => {
                        if self.tick_count % 5 != 0 {
                            continue;
                        }
                    }
                    ResourceCost::Low => {}
                }
            }

            let module = entry.module.clone();
            let res = catch_unwind(AssertUnwindSafe(|| {
                module.on_tick(ctx)
            }));

            match res {
                Ok(Ok(())) => {
                    if entry.error_count > 0 {
                        entry.error_count = 0;
                        entry.cooldown_duration = Duration::from_secs(2);
                    }
                    dirty = true;
                }
                Ok(Err(e)) => {
                    eprintln!("Module '{}' returned error: {}", module.id(), e);
                    Self::quarantine_module(entry, Some(e));
                }
                Err(panic_err) => {
                    let panic_msg = parse_panic_message(&panic_err);
                    eprintln!("Module '{}' panicked: {}", module.id(), panic_msg);
                    Self::quarantine_module(entry, Some(panic_msg));
                }
            }
        }

        dirty
    }

    fn quarantine_module(entry: &mut ModuleEntry, _msg: Option<String>) {
        entry.state = ModuleRuntimeState::Error;
        entry.error_count += 1;
        
        let multiplier = 2u32.pow(entry.error_count.saturating_sub(1));
        let next_cooldown = Duration::from_secs(2) * multiplier;
        entry.cooldown_duration = if next_cooldown > Duration::from_secs(60) {
            Duration::from_secs(60)
        } else {
            next_cooldown
        };
        entry.next_retry = Some(Instant::now() + entry.cooldown_duration);
    }

    // --- Dynamic Extension Loading (T036, T037, T038) ---

    #[cfg(feature = "community")]
    pub fn load_community_plugin(
        &mut self,
        manifest: &liem_sdk::ExtensionManifest,
        developer_mode: bool,
    ) -> Result<(), String> {
        // 1. Verify manifest trust tier permissions (T038)
        liem_sdk::security::validate_extension_trust(manifest, developer_mode)?;

        // 2. Open library and check SDK API version compatibility (T037)
        let path = std::path::Path::new(&manifest.entry_point);
        let lib = unsafe { libloading::Library::new(path) }
            .map_err(|e| format!("Failed to load dynamic library: {}", e))?;

        let api_version_fn: libloading::Symbol<unsafe extern "C" fn() -> u32> = unsafe {
            lib.get(b"liem_plugin_api_version")
        }.map_err(|e| format!("Constructor symbol 'liem_plugin_api_version' not found: {}", e))?;

        let version = unsafe { api_version_fn() };
        if version != 1 {
            return Err(format!("Mismatched SDK API version: expected 1, got {}", version));
        }

        // 3. Load dynamic plugin using SDK (T036)
        let plugin = unsafe { liem_sdk::loader::load_dynamic_plugin(path) }?;
        let safe_plugin = liem_sdk::loader::UnwindSafePlugin::new(plugin);
        safe_plugin.on_enable()?;

        self.plugins.push(Box::new(safe_plugin));
        Ok(())
    }

    // --- Remote Module Handlers (T039) ---

    pub fn register_remote_module(&mut self, id: &str, name: &str, widget_ids: Vec<String>, timeout_secs: u64) {
        // Avoid duplicate registrations
        self.entries.retain(|e| e.module.id() != id);

        let mut widgets = Vec::new();
        for w_id in widget_ids {
            widgets.push(WidgetDefinition {
                widget_id: Box::leak(w_id.clone().into_boxed_str()),
                name: "Remote Widget",
                flex_weight: 1.0,
            });
            self.remote_widget_data.insert(w_id, "".to_string());
        }

        let module = Arc::new(RemoteModule {
            id: id.to_string(),
            name: name.to_string(),
            widgets,
        });

        self.entries.push(ModuleEntry {
            module,
            state: ModuleRuntimeState::Running,
            error_count: 0,
            next_retry: None,
            cooldown_duration: Duration::from_secs(2),
            last_heartbeat: Some(Instant::now()),
            heartbeat_timeout: Some(Duration::from_secs(timeout_secs)),
        });
    }

    pub fn update_remote_heartbeat(&mut self, id: &str) {
        for entry in &mut self.entries {
            if entry.module.id() == id {
                entry.last_heartbeat = Some(Instant::now());
            }
        }
    }

    pub fn update_remote_data(&mut self, widget_id: &str, data: &str) {
        self.remote_widget_data.insert(widget_id.to_string(), data.to_string());
    }

    pub fn check_remote_timeouts(&mut self) {
        let now = Instant::now();
        let mut timed_out_ids = Vec::new();

        for entry in &self.entries {
            if let (Some(lh), Some(to)) = (entry.last_heartbeat, entry.heartbeat_timeout) {
                if now.duration_since(lh) > to {
                    timed_out_ids.push(entry.module.id().to_string());
                }
            }
        }

        for id in timed_out_ids {
            println!("Remote module '{}' exceeded heartbeat timeout! Auto-unregistering...", id);
            if let Some(pos) = self.entries.iter().position(|e| e.module.id() == id) {
                let entry = self.entries.remove(pos);
                for w in entry.module.widgets() {
                    self.remote_widget_data.remove(w.widget_id);
                }
            }
        }
    }
}

impl Default for ModuleManager {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_panic_message(panic_err: &(dyn Any + Send)) -> String {
    if let Some(s) = panic_err.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_err.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

// --- Remote Module Struct ---

pub struct RemoteModule {
    pub id: String,
    pub name: String,
    pub widgets: Vec<WidgetDefinition>,
}

impl RemoteModule {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Plugin for RemoteModule {
    fn on_enable(&self) -> Result<(), String> {
        Ok(())
    }
    fn on_disable(&self) -> Result<(), String> {
        Ok(())
    }
}

impl BarModule for RemoteModule {
    fn id(&self) -> &'static str {
        Box::leak(self.id.clone().into_boxed_str())
    }
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }
    fn widgets(&self) -> Vec<WidgetDefinition> {
        self.widgets.clone()
    }
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: self.id.clone(),
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            author: "Remote Developer".to_string(),
            api_version: 1,
            min_sdk_version: "0.1.0".to_string(),
            max_sdk_version: "2.0.0".to_string(),
            permissions: vec![crate::core::module::Permission::None],
            capabilities: vec![crate::core::module::Capability::Widget],
            dependencies: vec![],
        }
    }
    fn init_ui(&self, _widget_id: &str, _ui_handle: &slint::Window) -> Result<(), String> {
        Ok(())
    }
}
