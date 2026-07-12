use slint::Color;

use crate::core::config::ThemeConfig;
use crate::core::renderer::MainWindow;

/// Parse HEX or HSL color strings into a Slint Color object.
pub fn parse_color(s: &str) -> Color {
    let clean = s.trim().to_lowercase();
    if clean.starts_with('#') {
        let hex = &clean[1..];
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            return Color::from_rgb_u8(r, g, b);
        }
    }
    
    if clean.starts_with("hsl") {
        let parts: Vec<&str> = clean
            .trim_start_matches("hsl(")
            .trim_end_matches(')')
            .split(',')
            .map(|p| p.trim().trim_end_matches('%'))
            .collect();
        if parts.len() >= 3 {
            let h: f32 = parts[0].parse().unwrap_or(0.0);
            let s: f32 = parts[1].parse().unwrap_or(0.0) / 100.0;
            let l: f32 = parts[2].parse().unwrap_or(0.0) / 100.0;
            return hsl_to_color(h, s, l);
        }
    }
    
    Color::from_rgb_u8(26, 26, 36)
}

fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;
    
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    
    Color::from_rgb_u8(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

/// Dynamically map and apply ThemeConfig tokens to a MainWindow's exposed Slint properties.
pub fn apply_theme_to_window(window: &MainWindow, theme: &ThemeConfig) {
    if let Some(surf_str) = theme.colors.get("surface") {
        window.set_surface_color(slint::Brush::SolidColor(parse_color(surf_str)));
    }
    if let Some(border_str) = theme.colors.get("secondary") {
        window.set_border_color(slint::Brush::SolidColor(parse_color(border_str)));
    }
    if let Some(radius_val) = theme.radius.get("medium") {
        window.set_corner_radius(*radius_val as f32);
    }
    window.set_panel_opacity(theme.opacity);
}
