use std::fs;
use liem_bar::cli::validate_config;

#[test]
fn test_cli_valid_config() {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("liem_bar_valid_test.json");

    let valid_json = r#"{
        "schema_version": 1,
        "active_profile": "default",
        "profiles": {
            "default": {
                "bars": [
                    {
                        "monitor_id": "primary",
                        "position": "Top",
                        "layout_name": "standard"
                    }
                ]
            }
        },
        "layouts": {
            "standard": {
                "name": "standard",
                "root": {
                    "type": "Row",
                    "children": []
                }
            }
        },
        "themes": {},
        "manage_windows_taskbar": false
    }"#;

    fs::write(&config_path, valid_json).unwrap();
    let res = validate_config(&config_path);
    assert!(res.is_ok(), "Expected valid configuration to pass validation, got: {:?}", res);

    let _ = fs::remove_file(config_path);
}

#[test]
fn test_cli_invalid_profile() {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("liem_bar_invalid_profile_test.json");

    let invalid_json = r#"{
        "schema_version": 1,
        "active_profile": "nonexistent",
        "profiles": {
            "default": {
                "bars": []
            }
        },
        "layouts": {},
        "themes": {},
        "manage_windows_taskbar": false
    }"#;

    fs::write(&config_path, invalid_json).unwrap();
    let res = validate_config(&config_path);
    assert!(res.is_err(), "Expected invalid profile to return an error, got: {:?}", res);
    let err = res.err().unwrap();
    assert!(err.contains("active_profile") || err.contains("Active profile"), "Error should mention active_profile, got: {}", err);

    let _ = fs::remove_file(config_path);
}

#[test]
fn test_cli_missing_layout() {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("liem_bar_missing_layout_test.json");

    let invalid_json = r#"{
        "schema_version": 1,
        "active_profile": "default",
        "profiles": {
            "default": {
                "bars": [
                    {
                        "monitor_id": "primary",
                        "position": "Top",
                        "layout_name": "missing_layout"
                    }
                ]
            }
        },
        "layouts": {},
        "themes": {},
        "manage_windows_taskbar": false
    }"#;

    fs::write(&config_path, invalid_json).unwrap();
    let res = validate_config(&config_path);
    assert!(res.is_err(), "Expected missing layout to return an error, got: {:?}", res);
    let err = res.err().unwrap();
    assert!(err.contains("layout") || err.contains("Layout"), "Error should mention layout, got: {}", err);

    let _ = fs::remove_file(config_path);
}
