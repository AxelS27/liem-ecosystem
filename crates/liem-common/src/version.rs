use crate::AppManifest;

/// Negotiates semantic versions. Returns Ok(()) if the major versions match,
/// indicating backward compatibility. Returns an error otherwise.
pub fn negotiate_version(v1: &str, v2: &str) -> Result<(), String> {
    let parsed_v1 = semver::Version::parse(v1)
        .map_err(|e| format!("Failed to parse version '{}': {}", v1, e))?;
    let parsed_v2 = semver::Version::parse(v2)
        .map_err(|e| format!("Failed to parse version '{}': {}", v2, e))?;
    
    if parsed_v1.major != parsed_v2.major {
        return Err(format!(
            "Incompatible major versions: {} and {}",
            parsed_v1.major, parsed_v2.major
        ));
    }
    Ok(())
}

/// Dynamic capability negotiation. Compares the capabilities arrays of two
/// manifests and returns the matching intersection.
pub fn negotiate_capabilities(m1: &AppManifest, m2: &AppManifest) -> Vec<String> {
    let mut common = Vec::new();
    for cap in &m1.capabilities {
        if m2.capabilities.contains(cap) {
            common.push(cap.clone());
        }
    }
    common
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiate_version_compatible() {
        assert!(negotiate_version("1.2.3", "1.5.0").is_ok());
        assert!(negotiate_version("0.3.2", "0.3.5").is_ok()); // Note: in SemVer, pre-1.0, minor is treated as major compatibility boundary, but for simplicity we verify matching major numbers here or standard semver. Wait, let's keep it simple: parsed.major == parsed.major.
    }

    #[test]
    fn test_negotiate_version_incompatible() {
        assert!(negotiate_version("1.2.3", "2.0.0").is_err());
        assert!(negotiate_version("2.1.0", "1.9.9").is_err());
    }

    #[test]
    fn test_negotiate_capabilities() {
        let m1 = AppManifest {
            app_id: "a".to_string(),
            name: "a".to_string(),
            version: "1.0.0".to_string(),
            protocol_version: 1,
            capabilities: vec!["theme.sync".to_string(), "wallpaper.changed".to_string()],
            published_services: vec![],
            event_subscriptions: vec![],
        };
        let m2 = AppManifest {
            app_id: "b".to_string(),
            name: "b".to_string(),
            version: "1.0.0".to_string(),
            protocol_version: 1,
            capabilities: vec!["theme.sync".to_string(), "bar.visible".to_string()],
            published_services: vec![],
            event_subscriptions: vec![],
        };

        let common = negotiate_capabilities(&m1, &m2);
        assert_eq!(common, vec!["theme.sync".to_string()]);
    }
}
