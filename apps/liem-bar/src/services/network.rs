use std::net::TcpStream;
use std::time::Duration;

use crate::core::module::NetworkService;

pub struct Win32NetworkService;

impl Win32NetworkService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Win32NetworkService {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkService for Win32NetworkService {}

impl Win32NetworkService {
    /// Verify active internet connectivity.
    pub fn is_connected(&self) -> bool {
        // Attempt connection to Google Public DNS with a very brief timeout (100ms)
        let addr = "8.8.8.8:53".parse().unwrap();
        TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
    }
}
