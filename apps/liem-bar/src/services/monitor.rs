use std::sync::Mutex;
use std::sync::OnceLock;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};

#[derive(Debug, Clone)]
pub struct MonitorDetails {
    pub name: String,
    pub bounds: RECT,
    pub work_area: RECT,
    pub is_primary: bool,
}

pub fn enumerate_monitors() -> Vec<MonitorDetails> {
    unsafe {
        let mut monitors = Vec::new();
        let monitors_ptr = &mut monitors as *mut Vec<MonitorDetails> as isize;

        unsafe extern "system" fn monitor_enum_proc(
            monitor: HMONITOR,
            _hdc: HDC,
            _rect: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let monitors = &mut *(lparam.0 as *mut Vec<MonitorDetails>);
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

            if GetMonitorInfoW(monitor, &mut info as *mut MONITORINFOEXW as *mut _).as_bool() {
                let name_len = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
                let name = String::from_utf16_lossy(&info.szDevice[..name_len]);

                monitors.push(MonitorDetails {
                    name,
                    bounds: info.monitorInfo.rcMonitor,
                    work_area: info.monitorInfo.rcWork,
                    is_primary: (info.monitorInfo.dwFlags & 1) != 0, // MONITORINFOF_PRIMARY = 1
                });
            }
            BOOL(1)
        }

        let _ = EnumDisplayMonitors(
            HDC(0),
            None,
            Some(monitor_enum_proc),
            LPARAM(monitors_ptr),
        );
        monitors
    }
}

static LAST_MONITOR_FINGERPRINT: OnceLock<Mutex<String>> = OnceLock::new();

/// Detect if the connected screen count or positioning has changed since the last check.
pub fn check_monitor_changes() -> bool {
    let monitors = enumerate_monitors();
    let mut fingerprint = String::new();
    for m in &monitors {
        fingerprint.push_str(&format!(
            "{}:{}:{},{},{},{};",
            m.name,
            m.is_primary,
            m.bounds.left,
            m.bounds.top,
            m.bounds.right,
            m.bounds.bottom
        ));
    }
    
    let cell = LAST_MONITOR_FINGERPRINT.get_or_init(|| Mutex::new(fingerprint.clone()));
    let mut guard = cell.lock().unwrap();
    if *guard != fingerprint {
        *guard = fingerprint;
        true
    } else {
        false
    }
}
