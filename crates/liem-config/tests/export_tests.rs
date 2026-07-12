use liem_config::exporter::{export_to_json, import_from_json, export_to_yaml, import_from_yaml};
use liem_config::AppConfig;

#[test]
fn test_json_export_import_parity() {
    let mut config = AppConfig::default();
    config.developer_mode = true;
    config.local_settings.insert("theme".to_string(), serde_json::json!({"accent": "red"}));

    // Export to JSON
    let json_str = export_to_json(&config).expect("Failed to export JSON");
    assert!(json_str.contains("developer_mode"));

    // Import from JSON
    let imported: AppConfig = import_from_json(&json_str).expect("Failed to import JSON");
    assert_eq!(config, imported);
}

#[test]
fn test_yaml_export_import_parity() {
    let mut config = AppConfig::default();
    config.developer_mode = true;
    config.local_settings.insert("theme".to_string(), serde_json::json!({"accent": "blue"}));

    // Export to YAML
    let yaml_str = export_to_yaml(&config).expect("Failed to export YAML");
    assert!(yaml_str.contains("developer_mode"));

    // Import from YAML
    let imported: AppConfig = import_from_yaml(&yaml_str).expect("Failed to import YAML");
    assert_eq!(config, imported);
}

#[test]
fn test_invalid_import_handling() {
    // Invalid JSON
    let bad_json = "{ invalid }";
    assert!(import_from_json::<AppConfig>(bad_json).is_err());

    // Invalid YAML
    let bad_yaml = "invalid: : yaml";
    assert!(import_from_yaml::<AppConfig>(bad_yaml).is_err());
}
