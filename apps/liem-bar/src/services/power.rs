use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

use crate::core::module::BatteryService;

pub struct Win32BatteryService;

impl Win32BatteryService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Win32BatteryService {
    fn default() -> Self {
        Self::new()
    }
}

impl BatteryService for Win32BatteryService {
    fn is_on_ac(&self) -> bool {
        self.get_battery_info().map(|(_, ac, _)| ac).unwrap_or(true)
    }
    fn percent(&self) -> u8 {
        self.get_battery_info().map(|(p, _, _)| p).unwrap_or(100)
    }
    fn is_saver_active(&self) -> bool {
        self.get_battery_info().map(|(_, _, saver)| saver).unwrap_or(false)
    }
}

impl Win32BatteryService {
    pub fn get_battery_info(&self) -> Result<(u8, bool, bool), String> {
        let mut status = SYSTEM_POWER_STATUS::default();
        
        unsafe {
            let _ = GetSystemPowerStatus(&mut status).map_err(|e| e.to_string())?;
        }

        let percent = status.BatteryLifePercent;
        let is_ac = status.ACLineStatus == 1;
        let is_saver = status.SystemStatusFlag == 1;

        Ok((percent, is_ac, is_saver))
    }
}
