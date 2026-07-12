pub mod manifest;
pub mod loader;
pub mod security;

// Re-export common symbols for easier consumer imports
pub use manifest::{ExtensionManifest, SecurityTier, Plugin};
pub use loader::UnwindSafePlugin;
pub use security::validate_extension_trust;
