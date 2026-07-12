use std::sync::Mutex;
use slint::ComponentHandle;

slint::slint!{
    import { VerticalBox, HorizontalBox } from "std-widgets.slint";
    
    export component ThemeHud inherits Window {
        width: 380px;
        height: 220px;
        title: "Liem Ecosystem - Wallpaper Status HUD";
        background: #1e1e2e;
        
        in-out property <color> accent-color: #007acc;
        in-out property <string> transition_style: "fade";
        in-out property <string> current_wallpaper: "None";
        in-out property <string> active_status: "Running";
        
        VerticalBox {
            alignment: start;
            spacing: 14px;
            padding: 16px;
            
            Text {
                text: "Liem Wallpaper Status HUD";
                font-size: 18px;
                color: #cdd6f4;
                horizontal-alignment: center;
                font-weight: 700;
            }
            
            Rectangle {
                height: 2px;
                background: accent-color;
            }
            
            HorizontalBox {
                padding: 0px;
                spacing: 12px;
                alignment: center;
                
                Text { text: "Accent Color:"; color: #a6adc8; font-size: 13px; }
                Rectangle {
                    width: 18px;
                    height: 18px;
                    border-radius: 9px;
                    background: accent-color;
                }
                Text { 
                    text: transition_style; 
                    color: #cdd6f4; 
                    font-size: 13px; 
                    font-weight: 700; 
                }
            }
            
            HorizontalBox {
                padding: 0px;
                spacing: 8px;
                alignment: center;
                Text { text: "Status:"; color: #a6adc8; font-size: 13px; }
                Text { text: active_status; color: #a6e3a1; font-size: 13px; font-weight: 700; }
            }
            
            Text {
                text: "Wallpaper: " + current_wallpaper;
                color: #a6adc8;
                font-size: 11px;
                horizontal-alignment: center;
                wrap: word-wrap;
            }
        }
    }
}

pub static HUD_HANDLE: Mutex<Option<slint::Weak<ThemeHud>>> = Mutex::new(None);

pub fn spawn_theme_hud() {
    std::thread::spawn(|| {
        let hud = ThemeHud::new().unwrap();
        
        {
            let mut handle = HUD_HANDLE.lock().unwrap();
            *handle = Some(hud.as_weak());
        }
        
        hud.run().unwrap();
    });
}

pub fn parse_hex_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        slint::Color::from_rgb_u8(r, g, b)
    } else {
        slint::Color::from_rgb_u8(0, 122, 204) // default blue
    }
}
