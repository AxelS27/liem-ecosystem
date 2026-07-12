pub mod exporter;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(default)]
    pub developer_mode: bool,
    #[serde(default = "default_true")]
    pub theme_sync: bool,
    #[serde(default = "default_true")]
    pub state_sync: bool,
    pub shared_config_path: Option<PathBuf>,
    #[serde(default)]
    pub local_settings: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            developer_mode: false,
            theme_sync: true,
            state_sync: true,
            shared_config_path: None,
            local_settings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("Config file not found at: {0}")]
    FileNotFound(PathBuf),
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(ref path) = self.shared_config_path {
            if path.as_os_str().is_empty() {
                return Err(ConfigError::ValidationError(
                    "shared_config_path cannot be empty".to_string()
                ));
            }
        }
        Ok(())
    }
}

pub fn get_default_config_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map(|exe| {
            exe.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .map_err(|e| e.to_string())
}

pub fn load_config(path: &Path) -> Result<AppConfig, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::FileNotFound(path.to_path_buf()));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::IoError(e.to_string()))?;
    let config: AppConfig = serde_json::from_str(&content)
        .map_err(|e| ConfigError::ParseError(e.to_string()))?;
    
    config.validate()?;
    Ok(config)
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), ConfigError> {
    config.validate()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
    }
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| ConfigError::SerializationError(e.to_string()))?;
    std::fs::write(path, content)
        .map_err(|e| ConfigError::IoError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(!config.developer_mode);
        assert!(config.theme_sync);
        assert!(config.state_sync);
        assert!(config.shared_config_path.is_none());
        assert!(config.local_settings.is_empty());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_deserialize_partial() {
        let json_str = r#"
        {
            "developer_mode": true
        }
        "#;
        let config: AppConfig = serde_json::from_str(json_str).unwrap();
        assert!(config.developer_mode);
        assert!(config.theme_sync); // defaults to true
        assert!(config.state_sync); // defaults to true
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = std::env::temp_dir().join("liem_config_test");
        let config_file = temp_dir.join("config.json");

        let mut config = AppConfig::default();
        config.developer_mode = true;
        config.local_settings.insert("key".to_string(), serde_json::json!("value"));

        assert!(save_config(&config_file, &config).is_ok());
        
        let loaded = load_config(&config_file);
        assert!(loaded.is_ok());
        let loaded = loaded.unwrap();
        assert!(loaded.developer_mode);
        assert_eq!(loaded.local_settings.get("key"), Some(&serde_json::json!("value")));

        // Clean up
        let _ = std::fs::remove_file(config_file);
        let _ = std::fs::remove_dir(temp_dir);
    }
}
