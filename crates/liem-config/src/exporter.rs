use serde::{Deserialize, Serialize};

/// Export configuration struct to pretty-printed JSON string
pub fn export_to_json<T: Serialize>(config: &T) -> Result<String, String> {
    serde_json::to_string_pretty(config).map_err(|e| e.to_string())
}

/// Import configuration struct from JSON string
pub fn import_from_json<T: for<'de> Deserialize<'de>>(json_str: &str) -> Result<T, String> {
    serde_json::from_str(json_str).map_err(|e| e.to_string())
}

/// Export configuration struct to YAML string
pub fn export_to_yaml<T: Serialize>(config: &T) -> Result<String, String> {
    serde_yaml::to_string(config).map_err(|e| e.to_string())
}

/// Import configuration struct from YAML string
pub fn import_from_yaml<T: for<'de> Deserialize<'de>>(yaml_str: &str) -> Result<T, String> {
    serde_yaml::from_str(yaml_str).map_err(|e| e.to_string())
}
