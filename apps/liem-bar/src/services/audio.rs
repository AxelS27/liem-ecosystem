use std::sync::Mutex;

use crate::core::module::AudioService;

pub struct StatefulAudioService {
    volume: Mutex<u8>,
    muted: Mutex<bool>,
}

impl StatefulAudioService {
    pub fn new() -> Self {
        Self {
            volume: Mutex::new(60), // Default volume 60%
            muted: Mutex::new(false),
        }
    }
}

impl Default for StatefulAudioService {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioService for StatefulAudioService {}

impl StatefulAudioService {
    pub fn get_volume(&self) -> u8 {
        *self.volume.lock().unwrap()
    }

    pub fn set_volume(&self, volume: u8) {
        let mut v = self.volume.lock().unwrap();
        *v = volume.min(100);
    }

    pub fn is_muted(&self) -> bool {
        *self.muted.lock().unwrap()
    }

    pub fn set_muted(&self, muted: bool) {
        let mut m = self.muted.lock().unwrap();
        *m = muted;
    }
}
