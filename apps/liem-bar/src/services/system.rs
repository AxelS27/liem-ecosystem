use std::sync::Mutex;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::Foundation::FILETIME;
use windows::core::w;

use crate::core::module::SystemService;

fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

pub struct Win32SystemService {
    last_times: Mutex<Option<(u64, u64, u64)>>, // (idle, kernel, user)
}

impl Win32SystemService {
    pub fn new() -> Self {
        Self {
            last_times: Mutex::new(None),
        }
    }
}

impl Default for Win32SystemService {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemService for Win32SystemService {}

impl Win32SystemService {
    pub fn get_cpu_load(&self) -> Result<f32, String> {
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();

        unsafe {
            GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user))
                .map_err(|e| e.to_string())?;
        }

        let idle_val = filetime_to_u64(&idle);
        let kernel_val = filetime_to_u64(&kernel);
        let user_val = filetime_to_u64(&user);

        let mut guard = self.last_times.lock().unwrap();
        let load = if let Some((last_idle, last_kernel, last_user)) = *guard {
            let idle_delta = idle_val.saturating_sub(last_idle);
            let kernel_delta = kernel_val.saturating_sub(last_kernel);
            let user_delta = user_val.saturating_sub(last_user);
            let total_system = kernel_delta.saturating_add(user_delta);

            if total_system > 0 {
                let active = total_system.saturating_sub(idle_delta);
                (active as f32 / total_system as f32) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        *guard = Some((idle_val, kernel_val, user_val));
        Ok(load)
    }

    pub fn get_memory_info(&self) -> Result<(u64, u64), String> {
        let mut status = MEMORYSTATUSEX::default();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        unsafe {
            GlobalMemoryStatusEx(&mut status)
                .map_err(|e: windows::core::Error| e.to_string())?;
        }

        let total = status.ullTotalPhys;
        let free = status.ullAvailPhys;
        let used = total.saturating_sub(free);

        Ok((used, total))
    }

    pub fn get_storage_info(&self) -> Result<(u64, u64), String> {
        let mut free_bytes = 0u64;
        let mut total_bytes = 0u64;
        let mut total_free = 0u64;

        unsafe {
            GetDiskFreeSpaceExW(
                w!("C:\\"),
                Some(&mut free_bytes),
                Some(&mut total_bytes),
                Some(&mut total_free),
            ).map_err(|e| e.to_string())?;
        }

        let used = total_bytes.saturating_sub(free_bytes);
        Ok((used, total_bytes))
    }
}
