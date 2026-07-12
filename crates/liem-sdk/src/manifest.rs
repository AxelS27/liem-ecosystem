use serde::{Deserialize, Serialize};
use regex::Regex;
use std::sync::OnceLock;

/// Trust and security verification levels for extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecurityTier {
    /// Unsigned local extension, requires developer mode and explicit user consent
    Development,
    /// Automated registry checked extension
    Community,
    /// Cryptographically signed by a trusted publisher
    Trusted,
}

/// Manifest definition for Liem Desktop Ecosystem Extensions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionManifest {
    /// Unique reverse-DNS identifier (e.g., "com.author.plugin-name")
    pub id: String,
    /// User-friendly name
    pub name: String,
    /// Version string conforming to SemVer
    pub version: String,
    /// Author information
    pub author: String,
    /// Target path to script or binary DLL entry point
    pub entry_point: String,
    /// Declared security/trust tier
    pub security_tier: SecurityTier,
    /// Active state flag
    pub enabled: bool,
}

impl ExtensionManifest {
    /// Validates fields (e.g. reverse-DNS id and SemVer version)
    pub fn validate(&self) -> Result<(), String> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_-]+(?:\.[a-zA-Z0-9_-]+)+$").unwrap());
        
        if !re.is_match(&self.id) {
            return Err(format!(
                "Invalid extension ID '{}': must be a valid reverse-DNS pattern (e.g., com.author.extension)",
                self.id
            ));
        }

        semver::Version::parse(&self.version)
            .map_err(|e| format!("Invalid extension version '{}': {}", self.version, e))?;

        if self.name.trim().is_empty() {
            return Err("Extension name cannot be empty".to_string());
        }

        if self.entry_point.trim().is_empty() {
            return Err("Extension entry point cannot be empty".to_string());
        }

        Ok(())
    }
}

/// The core Plugin trait interfaces that extensions must implement.
/// All plugin methods are executed with panic containment to isolate errors.
pub trait Plugin: Send + Sync {
    /// Called when the plugin is enabled/loaded
    fn on_enable(&self) -> Result<(), String>;
    /// Called when the plugin is disabled/unloaded
    fn on_disable(&self) -> Result<(), String>;
}
