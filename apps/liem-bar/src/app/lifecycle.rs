use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use slint::ComponentHandle;
use liem_common::AppLifecycleState;

use crate::core::config::{load_or_create_bar_config, BarPosition, LayoutNode, ThemeConfig, LiemBarSettings};
use crate::core::renderer::{Renderer, SlintRenderer, MainWindow};
use crate::core::module_manager::ModuleManager;
use crate::core::module::{BarModule, ServiceContext};
use crate::modules::clock::ClockModule;
use crate::modules::calendar::CalendarModule;

// Win32 services
use crate::services::power::Win32BatteryService;
use crate::services::network::Win32NetworkService;
use crate::services::audio::StatefulAudioService;
use crate::services::system::Win32SystemService;

thread_local! {
    static RENDERER_CELL: RefCell<Option<SlintRenderer>> = RefCell::new(None);
}

struct WindowRenderer<'a, 'b>(&'a MainWindow, &'b std::collections::HashMap<String, String>);

impl<'a, 'b> Renderer for WindowRenderer<'a, 'b> {
    fn create_bar(
        &mut self,
        _monitor_id: &str,
        _position: BarPosition,
        _width: u32,
        _height: u32,
        _margin: u32,
    ) -> Result<(), String> {
        Ok(())
    }

    fn render_layout_tree(&mut self, root: &LayoutNode) -> Result<(), String> {
        let size = self.0.window().size();
        let w = size.width as f32;
        let h = size.height as f32;
        
        let positioned = crate::core::layout::evaluate_layout(root, 0.0, 0.0, w, h);
        
        let mut slint_widgets = Vec::new();
        for pw in positioned {
            let text = match pw.widget_id.as_str() {
                "clock.time" => self.0.get_clock_time().to_string(),
                "clock.date" => self.0.get_clock_date().to_string(),
                "calendar.grid" => self.0.get_calendar_text().to_string(),
                other => {
                    if let Some(data) = self.1.get(other) {
                        data.clone()
                    } else {
                        other.to_string()
                    }
                }
            };
            
            slint_widgets.push(crate::core::renderer::SlintWidget {
                widget_id: pw.widget_id.into(),
                x: pw.bounds_x,
                y: pw.bounds_y,
                width: pw.bounds_w,
                height: pw.bounds_h,
                text: text.into(),
            });
        }
        
        self.0.set_widgets(slint::ModelRc::new(slint::VecModel::from(slint_widgets)));
        Ok(())
    }

    fn apply_theme(&mut self, _theme: &ThemeConfig) -> Result<(), String> {
        Ok(())
    }

    fn set_visible(&mut self, _visible: bool) -> Result<(), String> {
        Ok(())
    }

    fn run(&mut self) -> Result<(), String> {
        Ok(())
    }
}

pub struct LiemBarApp {
    state: AppLifecycleState,
    config_path: PathBuf,
    settings: Option<LiemBarSettings>,
    module_manager: Arc<Mutex<ModuleManager>>,
    active_layout: Arc<Mutex<LayoutNode>>,
    timer: Option<slint::Timer>,

    // Win32 Services
    battery_service: Arc<Win32BatteryService>,
    network_service: Arc<Win32NetworkService>,
    audio_service: Arc<StatefulAudioService>,
    system_service: Arc<Win32SystemService>,
}

impl LiemBarApp {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            state: AppLifecycleState::Created,
            config_path,
            settings: None,
            module_manager: Arc::new(Mutex::new(ModuleManager::new())),
            active_layout: Arc::new(Mutex::new(LayoutNode::Spacer)),
            timer: None,
            battery_service: Arc::new(Win32BatteryService::new()),
            network_service: Arc::new(Win32NetworkService::new()),
            audio_service: Arc::new(StatefulAudioService::new()),
            system_service: Arc::new(Win32SystemService::new()),
        }
    }

    #[allow(dead_code)]
    pub fn state(&self) -> AppLifecycleState {
        self.state
    }

    pub fn bootstrap(&mut self) -> Result<(), String> {
        self.transition(AppLifecycleState::Initializing)?;

        // 1. Load configuration and run migrations if needed
        let (_app_config, bar_settings) = load_or_create_bar_config(&self.config_path)?;
        self.settings = Some(bar_settings.clone());

        // 2. Extract defaults for positioning
        let active_profile = &bar_settings.active_profile;
        let profile = bar_settings.profiles.get(active_profile)
            .ok_or_else(|| format!("Active profile '{}' not found", active_profile))?;

        // 3. Initialize the thread-local SlintRenderer and create windows
        RENDERER_CELL.with(|cell| {
            *cell.borrow_mut() = Some(SlintRenderer::new());
        });

        RENDERER_CELL.with(|cell| {
            let mut r_borrow = cell.borrow_mut();
            let renderer = r_borrow.as_mut().unwrap();
            for bar in &profile.bars {
                renderer.create_bar(
                    &bar.monitor_id,
                    bar.position.clone(),
                    1920, // default width
                    40,   // default height
                    0,    // default margin
                )?;
            }
            Ok::<(), String>(())
        })?;

        // 4. Instantiate modules (Clock and Calendar)
        let clock = Arc::new(ClockModule::new());
        let calendar = Arc::new(CalendarModule::new());
        
        {
            let mut manager = self.module_manager.lock().unwrap();
            manager.register_module(clock);
            manager.register_module(calendar);
        }

        // 5. Initialize UI mappings for each module across all spawned windows
        {
            let mut manager = self.module_manager.lock().unwrap();
            RENDERER_CELL.with(|cell| {
                let r_borrow = cell.borrow();
                let renderer = r_borrow.as_ref().unwrap();
                for window in renderer.get_windows().values() {
                    let slint_win = window.window();
                    for m in manager.get_modules() {
                        let _ = manager.init_ui(widget_id_or_empty(m.as_ref()), slint_win);
                    }
                }
            });
        }

        // 6. Find layout and initialize active layout
        let layout_name = &profile.bars[0].layout_name;
        let layout = bar_settings.layouts.get(layout_name)
            .ok_or_else(|| format!("Layout '{}' not found", layout_name))?
            .root.clone();

        *self.active_layout.lock().unwrap() = layout;

        // 7. Initialize configuration hot-reload watcher
        let config_path = self.config_path.clone();
        let layout_name_clone = layout_name.clone();
        let active_profile_clone = active_profile.clone();
        let active_layout_arc = self.active_layout.clone();
        crate::core::config::watch_config_file(&config_path, move |new_settings| {
            println!("Configuration changes detected! Hot-reloading active profile layouts...");
            if let Some(_profile) = new_settings.profiles.get(&active_profile_clone) {
                if let Some(layout_cfg) = new_settings.layouts.get(&layout_name_clone) {
                    let mut layout_guard = active_layout_arc.lock().unwrap();
                    *layout_guard = layout_cfg.root.clone();
                }
            }
        });

        // 8. Start Ecosystem event client sync loop (takes a list of weak handles dynamically)
        // Note: start_ecosystem_client uses the global ACTIVE_WINDOWS registry and is fully thread-safe
        #[cfg(feature = "ipc")]
        {
            // We spawn a mock/dummy Arc thread holder or call start_ecosystem_client directly
            // start_ecosystem_client doesn't take renderer anymore because it queries get_active_windows()
            crate::integrations::ecosystem::start_ecosystem_client(self.module_manager.clone());
        }

        // 9. Apply active profile theme styling
        if let Some(theme) = bar_settings.themes.get("default") {
            RENDERER_CELL.with(|cell| {
                let mut r_borrow = cell.borrow_mut();
                let _ = r_borrow.as_mut().unwrap().apply_theme(theme);
            });
        }

        // 10. If configured, enable auto-hide on the native Windows taskbar
        if bar_settings.manage_windows_taskbar {
            crate::platform::windows::set_windows_taskbar_autohide(true);
        }

        self.transition(AppLifecycleState::Ready)?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.transition(AppLifecycleState::Running)?;
        
        // Show all active windows
        RENDERER_CELL.with(|cell| {
            let mut r_borrow = cell.borrow_mut();
            let _ = r_borrow.as_mut().unwrap().set_visible(true);
        });

        // 1. Setup Slint Timer loop for ticks and hot-unplug checking
        let timer = slint::Timer::default();
        
        let manager_arc = self.module_manager.clone();
        let layout_arc = self.active_layout.clone();
        let settings = self.settings.clone().unwrap();

        // Clones of service handles for Context mapping
        let battery = self.battery_service.clone();
        let network = self.network_service.clone();
        let audio = self.audio_service.clone();
        let system = self.system_service.clone();

        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(100),
            move || {
                // Check if screen monitor details changed (unplug/plug)
                if crate::services::monitor::check_monitor_changes() {
                    println!("Monitor configuration changed! Dynamically adjusting active bars...");
                    
                    RENDERER_CELL.with(|cell| {
                        if let Some(ref mut renderer) = *cell.borrow_mut() {
                            // Dispose of existing windows
                            let monitor_ids: Vec<String> = renderer.get_windows().keys().cloned().collect();
                            for id in monitor_ids {
                                renderer.dispose_bar(&id);
                            }

                            // Rebuild windows for active monitors
                            let active_profile = &settings.active_profile;
                            if let Some(profile) = settings.profiles.get(active_profile) {
                                for bar in &profile.bars {
                                    let _ = renderer.create_bar(
                                        &bar.monitor_id,
                                        bar.position.clone(),
                                        1920,
                                        40,
                                        0,
                                    );
                                }
                            }

                            // Re-initialize modules on the newly spawned windows
                            let mut manager = manager_arc.lock().unwrap();
                            let modules = manager.get_modules();
                            for window in renderer.get_windows().values() {
                                let slint_win = window.window();
                                for m in &modules {
                                    for widget in m.widgets() {
                                        let _ = manager.init_ui(widget.widget_id, slint_win);
                                    }
                                }
                            }
                        }
                    });
                }

                // Proceed with normal tick delivery to modules
                RENDERER_CELL.with(|cell| {
                    if let Some(ref mut renderer) = *cell.borrow_mut() {
                        if let Some(window) = renderer.get_windows().values().next() {
                            let window_weak = window.as_weak();
                            if let Some(window) = window_weak.upgrade() {
                                let ctx = ServiceContext {
                                    battery: Some(battery.clone() as Arc<dyn crate::core::module::BatteryService>),
                                    network: Some(network.clone() as Arc<dyn crate::core::module::NetworkService>),
                                    audio: Some(audio.clone() as Arc<dyn crate::core::module::AudioService>),
                                    monitor: None,
                                    system: Some(system.clone() as Arc<dyn crate::core::module::SystemService>),
                                };

                                let mut manager = manager_arc.lock().unwrap();
                                let dirty = manager.tick(&ctx);
                                if dirty {
                                    let mut wr = WindowRenderer(&window, &manager.remote_widget_data);
                                    let layout = layout_arc.lock().unwrap().clone();
                                    let _ = wr.render_layout_tree(&layout);
                                }
                            }
                        }
                    }
                });
            },
        );

        self.timer = Some(timer);

        // 2. Block on Slint event loop without holding RENDERER_CELL borrow
        let window_weak = crate::core::renderer::get_active_windows().first().cloned();

        if let Some(weak) = window_weak {
            if let Some(window) = weak.upgrade() {
                window.run().map_err(|e| e.to_string())?;
            } else {
                return Err("Active window was already destroyed before running".to_string());
            }
        } else {
            return Err("No active windows to run".to_string());
        }

        // 3. Restore the native Windows taskbar if auto-hide was toggled
        if let Some(ref settings) = self.settings {
            if settings.manage_windows_taskbar {
                crate::platform::windows::set_windows_taskbar_autohide(false);
            }
        }

        self.transition(AppLifecycleState::Stopping)?;
        self.transition(AppLifecycleState::Stopped)?;
        Ok(())
    }

    fn transition(&mut self, new_state: AppLifecycleState) -> Result<(), String> {
        match (self.state, new_state) {
            (AppLifecycleState::Created, AppLifecycleState::Initializing) => {}
            (AppLifecycleState::Initializing, AppLifecycleState::Ready) => {}
            (AppLifecycleState::Ready, AppLifecycleState::Running) => {}
            (AppLifecycleState::Running, AppLifecycleState::Suspended) => {}
            (AppLifecycleState::Suspended, AppLifecycleState::Running) => {}
            (AppLifecycleState::Running, AppLifecycleState::Stopping) => {}
            (AppLifecycleState::Stopping, AppLifecycleState::Stopped) => {}
            _ => return Err(format!("Invalid state transition: {:?} -> {:?}", self.state, new_state)),
        }
        self.state = new_state;
        Ok(())
    }
}

fn widget_id_or_empty(m: &dyn BarModule) -> &str {
    if let Some(w) = m.widgets().first() {
        w.widget_id
    } else {
        ""
    }
}
