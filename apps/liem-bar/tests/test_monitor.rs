use windows::Win32::Foundation::RECT;
use liem_bar::core::config::BarPosition;

// Bounds calculation logic copied for validation check
fn calculate_bar_coords(
    bounds: RECT,
    position: BarPosition,
    width: u32,
    height: u32,
    margin: u32,
) -> (i32, i32, i32, i32) {
    let m_width = (bounds.right - bounds.left) as u32;
    let m_height = (bounds.bottom - bounds.top) as u32;

    match position {
        BarPosition::Top => {
            let x = bounds.left + margin as i32;
            let y = bounds.top + margin as i32;
            let w = m_width - (2 * margin);
            let h = height;
            (x, y, w as i32, h as i32)
        }
        BarPosition::Bottom => {
            let x = bounds.left + margin as i32;
            let y = bounds.bottom - height as i32 - margin as i32;
            let w = m_width - (2 * margin);
            let h = height;
            (x, y, w as i32, h as i32)
        }
        BarPosition::Left => {
            let x = bounds.left + margin as i32;
            let y = bounds.top + margin as i32;
            let w = width;
            let h = m_height - (2 * margin);
            (x, y, w as i32, h as i32)
        }
        BarPosition::Right => {
            let x = bounds.right - width as i32 - margin as i32;
            let y = bounds.top + margin as i32;
            let w = width;
            let h = m_height - (2 * margin);
            (x, y, w as i32, h as i32)
        }
    }
}

#[test]
fn test_primary_monitor_top_position() {
    let bounds = RECT {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };

    let (x, y, w, h) = calculate_bar_coords(bounds, BarPosition::Top, 0, 40, 10);
    assert_eq!(x, 10);
    assert_eq!(y, 10);
    assert_eq!(w, 1900);
    assert_eq!(h, 40);
}

#[test]
fn test_secondary_monitor_top_position_offset() {
    // Secondary monitor positioned to the right of primary
    let bounds = RECT {
        left: 1920,
        top: 0,
        right: 3840,
        bottom: 1080,
    };

    let (x, y, w, h) = calculate_bar_coords(bounds, BarPosition::Top, 0, 40, 15);
    assert_eq!(x, 1920 + 15);
    assert_eq!(y, 15);
    assert_eq!(w, 1920 - 30);
    assert_eq!(h, 40);
}

#[test]
fn test_secondary_monitor_bottom_position_offset() {
    // Secondary monitor positioned below primary
    let bounds = RECT {
        left: 0,
        top: 1080,
        right: 1920,
        bottom: 2160,
    };

    let (x, y, w, h) = calculate_bar_coords(bounds, BarPosition::Bottom, 0, 50, 20);
    assert_eq!(x, 20);
    assert_eq!(y, 2160 - 50 - 20);
    assert_eq!(w, 1920 - 40);
    assert_eq!(h, 50);
}
