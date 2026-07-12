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

use std::collections::HashMap;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CssStyle {
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub border_radius: Option<f32>,
    pub color: Option<String>,
    pub font_size: Option<f32>,
    pub opacity: Option<f32>,
}

/// A simple robust CSS parser to extract #selector properties.
pub fn parse_css(content: &str) -> HashMap<String, CssStyle> {
    let mut styles = HashMap::new();
    let mut current_selector = String::new();
    let mut in_block = false;
    let mut block_content = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if !in_block {
            if let Some(pos) = line.find('{') {
                current_selector = line[..pos].trim().trim_start_matches('#').to_string();
                in_block = true;
                let rest = line[pos + 1..].trim();
                if !rest.is_empty() {
                    block_content.push_str(rest);
                }
            } else {
                current_selector = line.trim_start_matches('#').to_string();
            }
        } else {
            if let Some(pos) = line.find('}') {
                block_content.push_str(" ");
                block_content.push_str(&line[..pos]);
                
                let mut style = CssStyle::default();
                for declaration in block_content.split(';') {
                    let parts: Vec<&str> = declaration.split(':').collect();
                    if parts.len() == 2 {
                        let prop = parts[0].trim().to_lowercase();
                        let val = parts[1].trim().to_string();
                        match prop.as_str() {
                            "background-color" => style.background_color = Some(val),
                            "border-color" => style.border_color = Some(val),
                            "border-radius" => {
                                let num_str: String = val.chars().filter(|c| c.is_digit(10) || *c == '.').collect();
                                if let Ok(f) = num_str.parse::<f32>() {
                                    style.border_radius = Some(f);
                                }
                            }
                            "color" => style.color = Some(val),
                            "font-size" => {
                                let num_str: String = val.chars().filter(|c| c.is_digit(10) || *c == '.').collect();
                                if let Ok(f) = num_str.parse::<f32>() {
                                    style.font_size = Some(f);
                                }
                            }
                            "opacity" => {
                                if let Ok(f) = val.parse::<f32>() {
                                    style.opacity = Some(f);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                styles.insert(current_selector.clone(), style);
                block_content.clear();
                in_block = false;

                let rest = line[pos + 1..].trim();
                if !rest.is_empty() {
                    if let Some(start_pos) = rest.find('{') {
                        current_selector = rest[..start_pos].trim().trim_start_matches('#').to_string();
                        in_block = true;
                        block_content.push_str(rest[start_pos + 1..].trim());
                    }
                }
            } else {
                block_content.push_str(" ");
                block_content.push_str(line);
            }
        }
    }
    styles
}

pub fn apply_css_to_window(window: &MainWindow, styles: &HashMap<String, CssStyle>) {
    let keys = vec!["bar".to_string(), "mainwindow".to_string(), "MainWindow".to_string()];
    for key in keys {
        if let Some(style) = styles.get(&key) {
            if let Some(ref bg_str) = style.background_color {
                window.set_surface_color(slint::Brush::SolidColor(parse_color(bg_str)));
            }
            if let Some(ref border_str) = style.border_color {
                window.set_border_color(slint::Brush::SolidColor(parse_color(border_str)));
            }
            if let Some(radius) = style.border_radius {
                window.set_corner_radius(radius);
            }
            if let Some(opacity) = style.opacity {
                window.set_panel_opacity(opacity);
            }
            break;
        }
    }
}

pub fn get_widget_style(
    styles: &HashMap<String, CssStyle>,
    widget_id: &str,
) -> (slint::Brush, slint::Brush, f32, slint::Color, f32) {
    let keys = vec![
        widget_id.to_string(),
        widget_id.replace('.', "-"),
        "widget".to_string(),
    ];

    let mut bg = None;
    let mut border = None;
    let mut radius = None;
    let mut color = None;
    let mut font_size = None;

    for key in keys {
        if let Some(style) = styles.get(&key) {
            if bg.is_none() {
                bg = style.background_color.clone();
            }
            if border.is_none() {
                border = style.border_color.clone();
            }
            if radius.is_none() {
                radius = style.border_radius;
            }
            if color.is_none() {
                color = style.color.clone();
            }
            if font_size.is_none() {
                font_size = style.font_size;
            }
        }
    }

    let bg_brush = slint::Brush::SolidColor(parse_color(&bg.unwrap_or_else(|| "#1a1a24".to_string())));
    let border_brush = slint::Brush::SolidColor(parse_color(&border.unwrap_or_else(|| "#333344".to_string())));
    let radius_val = radius.unwrap_or(4.0);
    let text_color = parse_color(&color.unwrap_or_else(|| "#d1d1e0".to_string()));
    let font_sz = font_size.unwrap_or(14.0);

    (bg_brush, border_brush, radius_val, text_color, font_sz)
}
