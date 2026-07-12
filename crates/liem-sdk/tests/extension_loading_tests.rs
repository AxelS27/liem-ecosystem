use liem_sdk::{ExtensionManifest, SecurityTier, Plugin, UnwindSafePlugin, validate_extension_trust};

#[test]
fn test_manifest_validation() {
    // Valid manifest
    let valid = ExtensionManifest {
        id: "com.axel.cool-widget".to_string(),
        name: "Cool Widget".to_string(),
        version: "1.0.4".to_string(),
        author: "Axel".to_string(),
        entry_point: "widget.dll".to_string(),
        security_tier: SecurityTier::Development,
        enabled: true,
    };
    assert!(valid.validate().is_ok());

    // Invalid ID (missing dots / not reverse-DNS)
    let invalid_id = ExtensionManifest {
        id: "cool-widget".to_string(),
        ..valid.clone()
    };
    assert!(invalid_id.validate().is_err());

    // Invalid Version (not SemVer)
    let invalid_ver = ExtensionManifest {
        version: "1.0".to_string(),
        ..valid.clone()
    };
    assert!(invalid_ver.validate().is_err());

    // Empty name
    let empty_name = ExtensionManifest {
        name: "".to_string(),
        ..valid.clone()
    };
    assert!(empty_name.validate().is_err());
}

#[test]
fn test_developer_mode_trust_validation() {
    let dev_manifest = ExtensionManifest {
        id: "com.axel.widget".to_string(),
        name: "Widget".to_string(),
        version: "0.1.0".to_string(),
        author: "Axel".to_string(),
        entry_point: "widget.dll".to_string(),
        security_tier: SecurityTier::Development,
        enabled: true,
    };

    // Blocked if developer mode is false
    assert!(validate_extension_trust(&dev_manifest, false).is_err());

    // Allowed if developer mode is true
    assert!(validate_extension_trust(&dev_manifest, true).is_ok());

    // Community and Trusted tier are allowed regardless of developer mode
    let community_manifest = ExtensionManifest {
        security_tier: SecurityTier::Community,
        ..dev_manifest.clone()
    };
    assert!(validate_extension_trust(&community_manifest, false).is_ok());
    assert!(validate_extension_trust(&community_manifest, true).is_ok());
}

struct PanickingPlugin;

impl Plugin for PanickingPlugin {
    fn on_enable(&self) -> Result<(), String> {
        panic!("Something went horribly wrong in on_enable!");
    }

    fn on_disable(&self) -> Result<(), String> {
        panic!("Panic in on_disable!");
    }
}

struct SuccessfulPlugin;

impl Plugin for SuccessfulPlugin {
    fn on_enable(&self) -> Result<(), String> {
        Ok(())
    }

    fn on_disable(&self) -> Result<(), String> {
        Err("Disable failed".to_string())
    }
}

#[test]
fn test_plugin_panic_isolation() {
    let panicker = Box::new(PanickingPlugin);
    let safe_panicker = UnwindSafePlugin::new(panicker);

    // Call on_enable and verify panic is caught and returns an Err
    let enable_res = safe_panicker.on_enable();
    assert!(enable_res.is_err());
    let err_msg = enable_res.err().unwrap();
    assert!(err_msg.contains("Plugin panicked during on_enable"));
    assert!(err_msg.contains("Something went horribly wrong"));

    // Call on_disable and verify panic is caught
    let disable_res = safe_panicker.on_disable();
    assert!(disable_res.is_err());
    assert!(disable_res.err().unwrap().contains("Panic in on_disable"));
}

#[test]
fn test_successful_plugin_execution() {
    let success = Box::new(SuccessfulPlugin);
    let safe_success = UnwindSafePlugin::new(success);

    assert!(safe_success.on_enable().is_ok());
    assert!(safe_success.on_disable().is_err());
}
