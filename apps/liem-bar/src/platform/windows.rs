use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABM_SETSTATE, APPBARDATA, ABS_ALWAYSONTOP, ABS_AUTOHIDE,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
use windows::core::w;

/// Toggle native Windows taskbar auto-hide mode programmatically.
pub fn set_windows_taskbar_autohide(autohide: bool) {
    let mut abd = APPBARDATA::default();
    abd.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    abd.hWnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };

    if abd.hWnd.0 != 0 {
        let state = if autohide { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP };
        abd.lParam = windows::Win32::Foundation::LPARAM(state as isize);
        unsafe {
            let _ = SHAppBarMessage(ABM_SETSTATE, &mut abd);
        }
    }
}
