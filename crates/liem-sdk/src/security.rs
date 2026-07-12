use crate::manifest::{ExtensionManifest, SecurityTier};

/// Validates if an extension is allowed to load based on developer mode rules.
/// Returns Ok(()) if allowed, or an Err(String) with the rejection reason.
pub fn validate_extension_trust(manifest: &ExtensionManifest, developer_mode: bool) -> Result<(), String> {
    manifest.validate()?;

    if manifest.security_tier == SecurityTier::Development && !developer_mode {
        return Err(format!(
            "Blocked loading development extension '{}'. Developer mode must be enabled to run unsigned extensions.",
            manifest.id
        ));
    }

    Ok(())
}
