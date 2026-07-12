use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use liem_config::{AppConfig, load_config, save_config};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BarPosition {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BarPlacementConfig {
    pub monitor_id: String,
    pub position: BarPosition,
    #[serde(default)]
    pub layout_name: Option<String>,
    #[serde(default)]
    pub auto_hide: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileConfig {
    pub bars: Vec<BarPlacementConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum LayoutNode {
    Row { children: Vec<LayoutNode> },
    Column { children: Vec<LayoutNode> },
    Group { children: Vec<LayoutNode> },
    Spacer,
    Widget { widget_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutConfig {
    pub name: String,
    pub root: LayoutNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationConfig {
    pub duration_ms: u32,
    pub easing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeConfig {
    pub name: String,
    pub colors: HashMap<String, String>,
    pub spacing: HashMap<String, u32>,
    pub radius: HashMap<String, u32>,
    pub opacity: f32,
    pub blur_radius: u32,
    pub animations: HashMap<String, AnimationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiemBarSettings {
    pub schema_version: u32,
    pub active_profile: String,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,
    #[serde(default, skip_serializing)]
    pub layouts: HashMap<String, LayoutConfig>,
    #[serde(default, skip_serializing)]
    pub themes: HashMap<String, ThemeConfig>,
    pub manage_windows_taskbar: bool,
    #[serde(skip)]
    pub styles: HashMap<String, crate::core::theme::CssStyle>,
}

impl Default for LiemBarSettings {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert(
            "default".to_string(),
            ProfileConfig {
                bars: vec![BarPlacementConfig {
                    monitor_id: "primary".to_string(),
                    position: BarPosition::Top,
                    layout_name: None,
                    auto_hide: false,
                }],
            },
        );

        let mut layouts = HashMap::new();
        layouts.insert(
            "default".to_string(),
            LayoutConfig {
                name: "default".to_string(),
                root: LayoutNode::Row {
                    children: vec![
                        LayoutNode::Widget { widget_id: "clock.time".to_string() },
                        LayoutNode::Spacer,
                        LayoutNode::Widget { widget_id: "clock.date".to_string() },
                    ],
                },
            },
        );

        let mut themes = HashMap::new();
        let mut colors = HashMap::new();
        colors.insert("surface".to_string(), "hsl(220, 10%, 10%)".to_string());
        colors.insert("surface_alt".to_string(), "hsl(220, 10%, 15%)".to_string());
        colors.insert("primary".to_string(), "hsl(220, 80%, 50%)".to_string());
        colors.insert("secondary".to_string(), "hsl(220, 20%, 40%)".to_string());

        let mut spacing = HashMap::new();
        spacing.insert("small".to_string(), 4);
        spacing.insert("medium".to_string(), 8);
        spacing.insert("large".to_string(), 16);

        let mut radius = HashMap::new();
        radius.insert("small".to_string(), 4);
        radius.insert("medium".to_string(), 8);

        let mut animations = HashMap::new();
        animations.insert(
            "slide".to_string(),
            AnimationConfig {
                duration_ms: 250,
                easing: "ease-out".to_string(),
            },
        );

        themes.insert(
            "default".to_string(),
            ThemeConfig {
                name: "default".to_string(),
                colors,
                spacing,
                radius,
                opacity: 0.95,
                blur_radius: 20,
                animations,
            },
        );

        Self {
            schema_version: 1,
            active_profile: "default".to_string(),
            profiles,
            layouts,
            themes,
            manage_windows_taskbar: false,
            styles: HashMap::new(),
        }
    }
}

/// Load configuration, running schema migration if necessary.
pub fn load_or_create_bar_config(path: &Path) -> Result<(AppConfig, LiemBarSettings), String> {
    // 1. Try reading the file and parsing as AppConfig
    let mut app_config = match load_config(path) {
        Ok(cfg) => cfg,
        Err(liem_config::ConfigError::FileNotFound(_)) => {
            // Generate default config
            let mut cfg = AppConfig::default();
            let bar_default = LiemBarSettings::default();
            cfg.local_settings.insert(
                "liem-bar".to_string(),
                serde_json::to_value(&bar_default).map_err(|e| e.to_string())?,
            );
            save_config(path, &cfg).map_err(|e| e.to_string())?;
            cfg
        }
        Err(e) => {
            // If it failed to parse as AppConfig, check if we can parse it as raw LiemBarSettings!
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(bar_settings) = serde_json::from_str::<LiemBarSettings>(&content) {
                    let mut cfg = AppConfig::default();
                    cfg.local_settings.insert(
                        "liem-bar".to_string(),
                        serde_json::to_value(&bar_settings).map_err(|e| e.to_string())?,
                    );
                    return post_process_loaded_config(path, cfg, bar_settings);
                }
            }
            return Err(e.to_string());
        }
    };

    let raw_settings = match app_config.local_settings.get("liem-bar") {
        Some(s) => s,
        None => {
            // Fallback: check if the file itself is a raw LiemBarSettings!
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(bar_settings) = serde_json::from_str::<LiemBarSettings>(&content) {
                    app_config.local_settings.insert(
                        "liem-bar".to_string(),
                        serde_json::to_value(&bar_settings).map_err(|e| e.to_string())?,
                    );
                    return post_process_loaded_config(path, app_config, bar_settings);
                }
            }
            return Err("Missing 'liem-bar' configuration key".to_string());
        }
    };

    let bar_settings: LiemBarSettings = serde_json::from_value(raw_settings.clone())
        .map_err(|e| format!("Failed to parse LiemBarSettings: {}", e))?;

    post_process_loaded_config(path, app_config, bar_settings)
}

fn post_process_loaded_config(
    path: &Path,
    app_config: AppConfig,
    mut bar_settings: LiemBarSettings,
) -> Result<(AppConfig, LiemBarSettings), String> {
    // Perform migrations if schema version is older
    if bar_settings.schema_version < 1 {
        bar_settings = migrate_config(bar_settings)?;
    }

    // Resolve active profile directory
    let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let profiles_dir = config_dir.join("profiles");
    let active_profile_dir = profiles_dir.join(&bar_settings.active_profile);

    // If active profile directory doesn't exist, check if we have inline profile configuration.
    if !active_profile_dir.exists() {
        if !bar_settings.profiles.is_empty() {
            // We have inline profiles (e.g. from legacy config or tests).
            // Do not force directory loading, just use the inline ones.
            return Ok((app_config, bar_settings));
        }

        std::fs::create_dir_all(&active_profile_dir).map_err(|e| e.to_string())?;

        // Write default layout.json
        let default_layout = serde_json::json!({
            "type": "Row",
            "children": [
                { "type": "Widget", "widget_id": "clock.time" },
                { "type": "Spacer" },
                { "type": "Widget", "widget_id": "clock.date" }
            ]
        });
        std::fs::write(
            active_profile_dir.join("layout.json"),
            serde_json::to_string_pretty(&default_layout).unwrap(),
        ).map_err(|e| e.to_string())?;

        // Write default style.css
        let default_style = r#"/* Liem Bar Waybar-like CSS styling sheet */
#bar {
    background-color: #111115;
    border-color: #22222a;
    border-radius: 8px;
    opacity: 0.95;
}

#widget {
    background-color: #1a1a24;
    border-color: #333344;
    border-radius: 4px;
    color: #d1d1e0;
    font-size: 14px;
}

#clock-time {
    background-color: #1c2538;
    border-color: #2b3a58;
    color: #61afef;
    font-size: 15px;
}
"#;
        std::fs::write(
            active_profile_dir.join("style.css"),
            default_style,
        ).map_err(|e| e.to_string())?;

        // Write default theme.json
        let default_theme = serde_json::json!({
            "name": "default",
            "colors": {
                "primary": "hsl(220, 80%, 50%)",
                "secondary": "hsl(220, 20%, 40%)",
                "surface": "hsl(220, 10%, 10%)",
                "surface_alt": "hsl(220, 10%, 15%)"
            },
            "spacing": {
                "large": 16,
                "medium": 8,
                "small": 4
            },
            "radius": {
                "medium": 8,
                "small": 4
            },
            "opacity": 0.95,
            "blur_radius": 20,
            "animations": {
                "slide": {
                    "duration_ms": 250,
                    "easing": "ease-out"
                }
            }
        });
        std::fs::write(
            active_profile_dir.join("theme.json"),
            serde_json::to_string_pretty(&default_theme).unwrap(),
        ).map_err(|e| e.to_string())?;
    }

    // Now, load Layout config from layout.json in active profile folder
    let layout_path = active_profile_dir.join("layout.json");
    let layout_str = std::fs::read_to_string(&layout_path).map_err(|e| format!("Failed to read active profile layout.json: {}", e))?;
    let layout_root: LayoutNode = serde_json::from_str(&layout_str).map_err(|e| format!("Failed to parse active profile layout.json: {}", e))?;
    bar_settings.layouts.insert(
        bar_settings.active_profile.clone(),
        LayoutConfig {
            name: bar_settings.active_profile.clone(),
            root: layout_root,
        },
    );

    // Now, load Theme config from theme.json in active profile folder
    let theme_path = active_profile_dir.join("theme.json");
    let theme_str = std::fs::read_to_string(&theme_path).map_err(|e| format!("Failed to read active profile theme.json: {}", e))?;
    let theme_root: ThemeConfig = serde_json::from_str(&theme_str).map_err(|e| format!("Failed to parse active profile theme.json: {}", e))?;
    bar_settings.themes.insert(
        bar_settings.active_profile.clone(),
        theme_root,
    );

    // Now, load CSS styles from style.css in active profile folder
    let css_path = active_profile_dir.join("style.css");
    let css_str = std::fs::read_to_string(&css_path).map_err(|e| format!("Failed to read active profile style.css: {}", e))?;
    bar_settings.styles = crate::core::theme::parse_css(&css_str);

    Ok((app_config, bar_settings))
}

fn migrate_config(mut settings: LiemBarSettings) -> Result<LiemBarSettings, String> {
    if settings.schema_version == 0 {
        settings.schema_version = 1;
    }
    Ok(settings)
}

/// Watches the config file and active profile files for changes, calling the callback on reload.
pub fn watch_config_file<F>(path: &Path, mut on_reload: F)
where
    F: FnMut(LiemBarSettings) + Send + 'static,
{
    let path = path.to_path_buf();
    tokio::spawn(async move {
        let mut last_modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok();

        let mut last_profile_modified = std::collections::HashMap::new();
        let mut first_run = true;

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let mut changed = false;

            // Check root config
            if let Ok(metadata) = std::fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if Some(modified) != last_modified {
                        last_modified = Some(modified);
                        if !first_run {
                            changed = true;
                        }
                    }
                }
            }

            // Load settings briefly to know what files to watch
            if let Ok((_, bar_settings)) = load_or_create_bar_config(&path) {
                let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
                let active_profile_dir = config_dir.join("profiles").join(&bar_settings.active_profile);
                
                let watch_files = vec![
                    active_profile_dir.join("layout.json"),
                    active_profile_dir.join("theme.json"),
                    active_profile_dir.join("style.css"),
                ];

                for file in watch_files {
                    if let Ok(m) = std::fs::metadata(&file) {
                        if let Ok(modified) = m.modified() {
                            let last_mod = last_profile_modified.get(&file).cloned();
                            if Some(modified) != last_mod {
                                last_profile_modified.insert(file.clone(), modified);
                                if !first_run {
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }

            if changed {
                if let Ok((_, new_settings)) = load_or_create_bar_config(&path) {
                    on_reload(new_settings);
                }
            }

            first_run = false;
        }
    });
}
