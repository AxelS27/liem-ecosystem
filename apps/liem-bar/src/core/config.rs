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
    pub layout_name: String,
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
    pub profiles: HashMap<String, ProfileConfig>,
    pub layouts: HashMap<String, LayoutConfig>,
    pub themes: HashMap<String, ThemeConfig>,
    pub manage_windows_taskbar: bool,
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
                    layout_name: "default".to_string(),
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
        }
    }
}

/// Load configuration, running schema migration if necessary.
pub fn load_or_create_bar_config(path: &Path) -> Result<(AppConfig, LiemBarSettings), String> {
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
        Err(e) => return Err(e.to_string()),
    };

    let raw_settings = app_config
        .local_settings
        .get("liem-bar")
        .ok_or_else(|| "Missing 'liem-bar' configuration key".to_string())?;

    let mut bar_settings: LiemBarSettings = serde_json::from_value(raw_settings.clone())
        .map_err(|e| format!("Failed to parse LiemBarSettings: {}", e))?;

    // Perform migrations if schema version is older
    if bar_settings.schema_version < 1 {
        bar_settings = migrate_config(bar_settings)?;
        app_config.local_settings.insert(
            "liem-bar".to_string(),
            serde_json::to_value(&bar_settings).map_err(|e| e.to_string())?,
        );
        save_config(path, &app_config).map_err(|e| e.to_string())?;
    }

    Ok((app_config, bar_settings))
}

fn migrate_config(mut settings: LiemBarSettings) -> Result<LiemBarSettings, String> {
    // Current migration: upgrade from version 0 to 1
    if settings.schema_version == 0 {
        settings.schema_version = 1;
        // e.g. add new default layouts/themes if missing, or reshape fields
    }
    Ok(settings)
}

/// Watches the config file for changes, calling the callback on reload.
pub fn watch_config_file<F>(path: &Path, mut on_reload: F)
where
    F: FnMut(LiemBarSettings) + Send + 'static,
{
    let path = path.to_path_buf();
    tokio::spawn(async move {
        let mut last_modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok();

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Ok(metadata) = std::fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if Some(modified) != last_modified {
                        last_modified = Some(modified);
                        if let Ok((_, new_settings)) = load_or_create_bar_config(&path) {
                            on_reload(new_settings);
                        }
                    }
                }
            }
        }
    });
}
