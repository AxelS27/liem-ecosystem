use crate::error::LwError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub wallpaper_dir: PathBuf,
    pub shuffle: bool,
    pub transition_default: TransitionConfig,
    pub scheduler: SchedulerConfig,
    pub position: WallpaperPosition,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EasingStyle {
    Linear,
    Sine,
    Quad,
    Cubic,
    Quart,
    Quint,
    Exponential,
    Circular,
    Back,
    Bounce,
    Elastic,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EasingDirection {
    In,
    Out,
    InOut,
}

fn default_easing_style() -> EasingStyle {
    EasingStyle::Quad
}

fn default_easing_direction() -> EasingDirection {
    EasingDirection::InOut
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransitionConfig {
    pub effect_type: String,
    pub duration_secs: f32,
    #[serde(default = "default_easing_style")]
    pub easing_style: EasingStyle,
    #[serde(default = "default_easing_direction")]
    pub easing_direction: EasingDirection,
    pub target_fps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub interval_mins: u32,
    pub change_on_startup: bool,
    pub run_on_startup: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WallpaperPosition {
    Fill,
    Fit,
    Stretch,
    Tile,
    Center,
    Span,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wallpaper_dir: PathBuf::new(),
            shuffle: false,
            transition_default: TransitionConfig::default(),
            scheduler: SchedulerConfig::default(),
            position: WallpaperPosition::Fill,
        }
    }
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            effect_type: "fade".to_string(),
            duration_secs: 1.0,
            easing_style: EasingStyle::Quad,
            easing_direction: EasingDirection::InOut,
            target_fps: Some(60),
        }
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { enabled: false, interval_mins: 15, change_on_startup: true, run_on_startup: true }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), LwError> {
        // Validation check for wallpaper directory path
        // We only check if it is non-empty. If it's empty/default, we might allow it (e.g. before initial configuration is set)
        // but if it is set, it must exist.
        if !self.wallpaper_dir.as_os_str().is_empty() && !self.wallpaper_dir.exists() {
            return Err(LwError::Config(format!(
                "wallpaper_dir does not exist: {}",
                self.wallpaper_dir.display()
            )));
        }
        self.transition_default.validate()?;
        self.scheduler.validate()?;
        Ok(())
    }

    pub fn default_path() -> std::path::PathBuf {
        if let Ok(mut exe_path) = std::env::current_exe() {
            exe_path.pop();
            exe_path.join("config.json")
        } else {
            std::path::PathBuf::from("config.json")
        }
    }

    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, LwError> {
        let app_config = liem_config::load_config(path.as_ref()).map_err(|e| {
            LwError::Serialization(format!("Failed to load config JSON: {e}"))
        })?;
        
        let local_val = serde_json::to_value(&app_config.local_settings).map_err(|e| {
            LwError::Serialization(format!("Failed to convert config to value: {e}"))
        })?;
        
        let config: Config = serde_json::from_value(local_val).map_err(|e| {
            LwError::Serialization(format!("Failed to deserialize Config from local_settings: {e}"))
        })?;
        
        config.validate()?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), LwError> {
        self.validate()?;
        
        let mut app_config = liem_config::load_config(path.as_ref()).unwrap_or_default();
        
        let local_val = serde_json::to_value(self).map_err(|e| {
            LwError::Serialization(format!("Failed to serialize Config to value: {e}"))
        })?;
        
        let local_map: std::collections::HashMap<String, serde_json::Value> = serde_json::from_value(local_val).map_err(|e| {
            LwError::Serialization(format!("Failed to map Config to local_settings: {e}"))
        })?;
        
        app_config.local_settings = local_map;
        
        liem_config::save_config(path.as_ref(), &app_config).map_err(|e| {
            LwError::Serialization(format!("Failed to save config JSON: {e}"))
        })?;
        
        Ok(())
    }
}

impl TransitionConfig {
    pub fn validate(&self) -> Result<(), LwError> {
        if self.duration_secs < 0.1 || self.duration_secs > 10.0 {
            return Err(LwError::Config(format!(
                "duration_secs must be between 0.1 and 10.0, got {}",
                self.duration_secs
            )));
        }
        if let Some(fps) = self.target_fps {
            if fps == 0 || fps > 360 {
                return Err(LwError::Config(format!(
                    "target_fps must be between 1 and 360, got {}",
                    fps
                )));
            }
        }
        Ok(())
    }
}

impl SchedulerConfig {
    pub fn validate(&self) -> Result<(), LwError> {
        if self.interval_mins < 1 {
            return Err(LwError::Config(format!(
                "interval_mins must be greater than or equal to 1, got {}",
                self.interval_mins
            )));
        }
        Ok(())
    }
}
