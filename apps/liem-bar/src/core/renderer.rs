use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongW, SetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::core::config::{BarPosition, LayoutNode, ThemeConfig};
use crate::services::monitor::enumerate_monitors;

pub trait Renderer {
    /// Initialize a desktop bar window for the specified monitor.
    fn create_bar(
        &mut self,
        monitor_id: &str,
        position: BarPosition,
        width: u32,
        height: u32,
        margin: u32,
        auto_hide: bool,
    ) -> Result<(), String>;

    /// Render or update the hierarchical layout tree on the screen.
    fn render_layout_tree(&mut self, root: &LayoutNode) -> Result<(), String>;

    /// Update visual styling using design token themes (colors, corners, padding, opacity).
    fn apply_theme(&mut self, theme: &ThemeConfig) -> Result<(), String>;

    /// Apply CSS styles dynamically.
    fn apply_css(&mut self, styles: &std::collections::HashMap<String, crate::core::theme::CssStyle>) -> Result<(), String>;

    /// Toggle the visibility of the bar window (e.g. for slide-in/slide-out animations).
    fn set_visible(&mut self, visible: bool) -> Result<(), String>;

    /// Run the rendering event loop blocking thread.
    fn run(&mut self) -> Result<(), String>;
}

slint::include_modules!();

static WINDOW_REGISTRY: OnceLock<Mutex<HashMap<isize, slint::Weak<MainWindow>>>> = OnceLock::new();
static ACTIVE_WINDOWS: OnceLock<Mutex<Vec<slint::Weak<MainWindow>>>> = OnceLock::new();

pub fn register_active_window(weak: slint::Weak<MainWindow>) {
    let list = ACTIVE_WINDOWS.get_or_init(|| Mutex::new(Vec::new()));
    list.lock().unwrap().push(weak);
}

pub fn get_active_windows() -> Vec<slint::Weak<MainWindow>> {
    let list = ACTIVE_WINDOWS.get_or_init(|| Mutex::new(Vec::new()));
    list.lock().unwrap().clone()
}

pub fn clear_active_windows() {
    let list = ACTIVE_WINDOWS.get_or_init(|| Mutex::new(Vec::new()));
    list.lock().unwrap().clear();
}

pub fn get_hwnd(window: &slint::Window) -> Option<isize> {
    let window_handle = window.window_handle();
    if let Ok(handle) = window_handle.window_handle() {
        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            return Some(win32_handle.hwnd.get() as isize);
        }
    }
    None
}

pub fn register_window(window: &slint::Window, weak: slint::Weak<MainWindow>) {
    if let Some(hwnd) = get_hwnd(window) {
        let registry = WINDOW_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        registry.lock().unwrap().insert(hwnd, weak);
    }
}

pub fn get_window_component(window: &slint::Window) -> Option<slint::Weak<MainWindow>> {
    let hwnd = get_hwnd(window)?;
    let registry = WINDOW_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    registry.lock().unwrap().get(&hwnd).cloned()
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AutoHideState {
    Expanded,
    Collapsed,
    SlidingIn,
    SlidingOut,
}

pub struct SlintRenderer {
    windows: HashMap<String, MainWindow>,
    pub styles: HashMap<String, crate::core::theme::CssStyle>,
}

impl SlintRenderer {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            styles: HashMap::new(),
        }
    }

    pub fn get_windows(&self) -> &HashMap<String, MainWindow> {
        &self.windows
    }

    pub fn dispose_bar(&mut self, monitor_id: &str) {
        if let Some(window) = self.windows.remove(monitor_id) {
            let _ = window.hide();
        }
        clear_active_windows();
        for w in self.windows.values() {
            register_active_window(w.as_weak());
        }
    }
}

impl Default for SlintRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for SlintRenderer {
    fn create_bar(
        &mut self,
        monitor_id: &str,
        position: BarPosition,
        width: u32,
        height: u32,
        margin: u32,
        auto_hide: bool,
    ) -> Result<(), String> {
        let window = MainWindow::new().map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        register_active_window(window.as_weak());

        // 1. Enumerate monitors and locate matching display geometry
        let monitors = enumerate_monitors();
        let target_monitor = if monitor_id == "primary" {
            monitors.iter().find(|m| m.is_primary)
        } else {
            monitors.iter().find(|m| m.name.contains(monitor_id))
        };

        let monitor = target_monitor.ok_or_else(|| {
            format!("Monitor '{}' not found. Available: {:?}", monitor_id, monitors)
        })?.clone();

        // 2. Calculate coordinates based on monitor bounds, position and margin
        let m_width = (monitor.bounds.right - monitor.bounds.left) as u32;
        let m_height = (monitor.bounds.bottom - monitor.bounds.top) as u32;

        let (x, y, w, h) = match position {
            BarPosition::Top => {
                let x = monitor.bounds.left + margin as i32;
                let y = monitor.bounds.top + margin as i32;
                let w = m_width - (2 * margin);
                let h = height;
                (x, y, w as i32, h as i32)
            }
            BarPosition::Bottom => {
                let x = monitor.bounds.left + margin as i32;
                let y = monitor.bounds.bottom - height as i32 - margin as i32;
                let w = m_width - (2 * margin);
                let h = height;
                (x, y, w as i32, h as i32)
            }
            BarPosition::Left => {
                let x = monitor.bounds.left + margin as i32;
                let y = monitor.bounds.top + margin as i32;
                let w = width;
                let h = m_height - (2 * margin);
                (x, y, w as i32, h as i32)
            }
            BarPosition::Right => {
                let x = monitor.bounds.right - width as i32 - margin as i32;
                let y = monitor.bounds.top + margin as i32;
                let w = width;
                let h = m_height - (2 * margin);
                (x, y, w as i32, h as i32)
            }
        };

        // 3. Defer Win32 styling and HWND registration until event loop realization
        let bounds_rect = monitor.bounds;

        fn try_setup_window(
            weak_window: slint::Weak<MainWindow>,
            bounds_rect: RECT,
            position: BarPosition,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            auto_hide: bool,
        ) {
            if let Some(window) = weak_window.upgrade() {
                let slint_window = window.window();
                let window_handle = slint_window.window_handle();
                match window_handle.window_handle() {
                    Ok(handle) => {
                        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                            let hwnd = HWND(win32_handle.hwnd.get() as isize);
                            
                            // Register window globally with HWND mapping
                            register_window(window.window(), weak_window.clone());

                            unsafe {
                                // Apply WS_EX_TOOLWINDOW (prevent taskbar button) and WS_EX_TOPMOST style flags
                                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                                let new_style = ex_style | WS_EX_TOOLWINDOW.0 as i32 | WS_EX_TOPMOST.0 as i32;
                                SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);

                                // Position and size the window topmost at monitor bounds
                                let _ = SetWindowPos(
                                    hwnd,
                                    HWND_TOPMOST,
                                    x,
                                    y,
                                    w,
                                    h,
                                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                                );
                            }

                            // Spawn background loop for Auto-Hide detection and slide animation only if auto-hide is enabled
                            if auto_hide {
                                let hwnd_val = hwnd.0;
                                tokio::spawn(async move {
                                    let hwnd = HWND(hwnd_val);
                                    let mut state = AutoHideState::Expanded;
                                    let mut current_offset = 0i32;
                                    let max_offset = h - 2;

                                    loop {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                                        let mut pt = POINT::default();
                                        if unsafe { GetCursorPos(&mut pt) }.is_ok() {
                                            let in_monitor = pt.x >= bounds_rect.left && pt.x <= bounds_rect.right;
                                            
                                            match position {
                                                BarPosition::Top => {
                                                    let over_trigger = pt.y >= bounds_rect.top && pt.y <= bounds_rect.top + 4;
                                                    let over_bar = pt.y >= y && pt.y <= y + h;

                                                    match state {
                                                        AutoHideState::Expanded => {
                                                            if !over_bar && in_monitor {
                                                                state = AutoHideState::SlidingOut;
                                                            }
                                                        }
                                                        AutoHideState::Collapsed => {
                                                            if over_trigger && in_monitor {
                                                                state = AutoHideState::SlidingIn;
                                                            }
                                                        }
                                                        AutoHideState::SlidingOut => {
                                                            if current_offset < max_offset {
                                                                current_offset += 4;
                                                                if current_offset > max_offset {
                                                                    current_offset = max_offset;
                                                                }
                                                                let target_y = y - current_offset;
                                                                unsafe {
                                                                    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, target_y, w, h, SWP_NOACTIVATE);
                                                                }
                                                            } else {
                                                                state = AutoHideState::Collapsed;
                                                            }
                                                        }
                                                        AutoHideState::SlidingIn => {
                                                            if current_offset > 0 {
                                                                current_offset -= 4;
                                                                if current_offset < 0 {
                                                                    current_offset = 0;
                                                                }
                                                                let target_y = y - current_offset;
                                                                unsafe {
                                                                    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, target_y, w, h, SWP_NOACTIVATE);
                                                                }
                                                            } else {
                                                                state = AutoHideState::Expanded;
                                                            }
                                                        }
                                                    }
                                                }
                                                BarPosition::Bottom => {
                                                    let over_trigger = pt.y >= bounds_rect.bottom - 4 && pt.y <= bounds_rect.bottom;
                                                    let over_bar = pt.y >= y && pt.y <= y + h;

                                                    match state {
                                                        AutoHideState::Expanded => {
                                                            if !over_bar && in_monitor {
                                                                state = AutoHideState::SlidingOut;
                                                            }
                                                        }
                                                        AutoHideState::Collapsed => {
                                                            if over_trigger && in_monitor {
                                                                state = AutoHideState::SlidingIn;
                                                            }
                                                        }
                                                        AutoHideState::SlidingOut => {
                                                            if current_offset < max_offset {
                                                                current_offset += 4;
                                                                if current_offset > max_offset {
                                                                    current_offset = max_offset;
                                                                }
                                                                let target_y = y + current_offset;
                                                                unsafe {
                                                                    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, target_y, w, h, SWP_NOACTIVATE);
                                                                }
                                                            } else {
                                                                state = AutoHideState::Collapsed;
                                                            }
                                                        }
                                                        AutoHideState::SlidingIn => {
                                                            if current_offset > 0 {
                                                                current_offset -= 4;
                                                                if current_offset < 0 {
                                                                    current_offset = 0;
                                                                }
                                                                let target_y = y + current_offset;
                                                                unsafe {
                                                                    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, target_y, w, h, SWP_NOACTIVATE);
                                                                }
                                                            } else {
                                                                state = AutoHideState::Expanded;
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    Err(_) => {
                        // Handle is not ready yet, retry in 20 milliseconds
                        let weak_window_retry = weak_window.clone();
                        slint::Timer::single_shot(std::time::Duration::from_millis(20), move || {
                            try_setup_window(
                                weak_window_retry,
                                bounds_rect,
                                position,
                                x,
                                y,
                                w,
                                h,
                                auto_hide,
                            );
                        });
                    }
                }
            }
        }

        let weak_window_init = window.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(10), move || {
            try_setup_window(
                weak_window_init,
                bounds_rect,
                position,
                x,
                y,
                w,
                h,
                auto_hide,
            );
        });

        self.windows.insert(monitor_id.to_string(), window);
        Ok(())
    }

    fn render_layout_tree(&mut self, root: &LayoutNode) -> Result<(), String> {
        for window in self.windows.values() {
            let size = window.window().size();
            let w = size.width as f32;
            let h = size.height as f32;

            let positioned = crate::core::layout::evaluate_layout(root, 0.0, 0.0, w, h);

            let mut slint_widgets = Vec::new();
            for pw in positioned {
                let text = match pw.widget_id.as_str() {
                    "clock.time" => window.get_clock_time().to_string(),
                    "clock.date" => window.get_clock_date().to_string(),
                    "calendar.grid" => window.get_calendar_text().to_string(),
                    _ => pw.widget_id.clone(),
                };

                let (bg, border, radius, text_color, font_sz) = crate::core::theme::get_widget_style(&self.styles, &pw.widget_id);

                slint_widgets.push(SlintWidget {
                    widget_id: pw.widget_id.into(),
                    x: pw.bounds_x,
                    y: pw.bounds_y,
                    width: pw.bounds_w,
                    height: pw.bounds_h,
                    text: text.into(),
                    background_color: bg,
                    border_color: border,
                    border_radius: radius,
                    text_color,
                    font_size: font_sz,
                });
            }

            let model = slint::ModelRc::new(slint::VecModel::from(slint_widgets));
            window.set_widgets(model);
        }
        Ok(())
    }

    fn apply_theme(&mut self, theme: &ThemeConfig) -> Result<(), String> {
        for window in self.windows.values() {
            crate::core::theme::apply_theme_to_window(window, theme);
        }
        Ok(())
    }

    fn apply_css(&mut self, styles: &std::collections::HashMap<String, crate::core::theme::CssStyle>) -> Result<(), String> {
        self.styles = styles.clone();
        for window in self.windows.values() {
            crate::core::theme::apply_css_to_window(window, styles);
        }
        Ok(())
    }

    fn set_visible(&mut self, visible: bool) -> Result<(), String> {
        for window in self.windows.values() {
            if visible {
                window.show().map_err(|e| e.to_string())?;
            } else {
                window.hide().map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn run(&mut self) -> Result<(), String> {
        if let Some(window) = self.windows.values().next() {
            window.run().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("No active windows to run".to_string())
        }
    }
}
