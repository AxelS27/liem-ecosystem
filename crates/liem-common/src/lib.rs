pub mod version;

use serde::{Deserialize, Serialize};
use regex::Regex;
use std::sync::OnceLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLifecycleState {
    Created,
    Initializing,
    Ready,
    Running,
    Suspended,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppManifest {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub protocol_version: u32,
    pub capabilities: Vec<String>,
    pub published_services: Vec<String>,
    pub event_subscriptions: Vec<String>,
}

impl AppManifest {
    pub fn validate(&self) -> Result<(), String> {
        // Validation Rule 1: app_id must match pattern ^[a-zA-Z0-9._-]+$
        static APP_ID_REGEX: OnceLock<Regex> = OnceLock::new();
        let re = APP_ID_REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap());
        if !re.is_match(&self.app_id) {
            return Err(format!(
                "Invalid app_id '{}': must match pattern ^[a-zA-Z0-9._-]+$",
                self.app_id
            ));
        }

        // Validation Rule 2: version must be a valid SemVer representation
        if semver::Version::parse(&self.version).is_err() {
            return Err(format!(
                "Invalid version '{}': must be a valid SemVer representation",
                self.version
            ));
        }

        // Validation Rule 3: capabilities must be namespace-prefixed (contain a dot, e.g. "wallpaper.current")
        for cap in &self.capabilities {
            if !cap.contains('.') {
                return Err(format!(
                    "Invalid capability '{}': must be namespace-prefixed (e.g. 'wallpaper.current')",
                    cap
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEnvelope {
    pub sender_id: String,
    pub message_id: String,
    pub timestamp: DateTime<Utc>,
    pub channel: EventBusChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventBusChannel {
    Publish(EventPayload),
    Subscribe(SubscribePayload),
    Broadcast(EventPayload),
    Request(RequestPayload),
    Response(ResponsePayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    pub topic: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribePayload {
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPayload {
    pub service: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePayload {
    pub request_id: String,
    pub result: Result<serde_json::Value, IpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum IpcError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    #[error("Timeout")]
    Timeout,
    #[error("Internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest() {
        let manifest = AppManifest {
            app_id: "org.liem.wallpaper".to_string(),
            name: "Liem Wallpaper".to_string(),
            version: "1.0.3".to_string(),
            protocol_version: 1,
            capabilities: vec!["wallpaper.current".to_string()],
            published_services: vec![],
            event_subscriptions: vec![],
        };
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_invalid_app_id() {
        let manifest = AppManifest {
            app_id: "org/liem/wallpaper".to_string(),
            name: "Liem Wallpaper".to_string(),
            version: "1.0.0".to_string(),
            protocol_version: 1,
            capabilities: vec![],
            published_services: vec![],
            event_subscriptions: vec![],
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_invalid_version() {
        let manifest = AppManifest {
            app_id: "org.liem.wallpaper".to_string(),
            name: "Liem Wallpaper".to_string(),
            version: "1.0".to_string(), // Invalid SemVer
            protocol_version: 1,
            capabilities: vec![],
            published_services: vec![],
            event_subscriptions: vec![],
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_invalid_capability() {
        let manifest = AppManifest {
            app_id: "org.liem.wallpaper".to_string(),
            name: "Liem Wallpaper".to_string(),
            version: "1.0.0".to_string(),
            protocol_version: 1,
            capabilities: vec!["wallpaper_current".to_string()], // Missing dot
            published_services: vec![],
            event_subscriptions: vec![],
        };
        let res = manifest.validate();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("namespace-prefixed"));
    }
}
