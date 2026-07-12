use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::core::config::LiemBarSettings;
use crate::services::monitor::enumerate_monitors;
use crate::services::power::Win32BatteryService;

/// Route and run CLI subcommands.
pub fn run_cli_command(args: &[String], config_path: &Path) -> Result<(), String> {
    if args.is_empty() {
        return Err("No command specified. Available: validate, diagnostics, edit, update".to_string());
    }

    match args[0].as_str() {
        "validate" => validate_config(config_path),
        "diagnostics" => run_diagnostics(config_path),
        "edit" => edit_active_profile(config_path),
        "update" => run_update(),
        cmd => Err(format!(
            "Unknown command '{}'. Available: validate, diagnostics, edit, update",
            cmd
        )),
    }
}

fn run_update() -> Result<(), String> {
    println!("Checking for ecosystem updates via Liem Wallpaper...");
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    
    let mut target = exe_path.join("lw.exe");
    if !target.exists() {
        if let Some(parent) = exe_path.parent() {
            target = parent.join("Liem Wallpaper").join("lw.exe");
        }
    }
    if !target.exists() {
        target = std::path::PathBuf::from("lw.exe");
    }
    
    let mut child = std::process::Command::new(target)
        .arg("update")
        .spawn()
        .map_err(|e| format!("Failed to execute update command: {}", e))?;
        
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("Update failed with exit code: {:?}", status.code()));
    }
    Ok(())
}

/// Validate configuration integrity.
pub fn validate_config(config_path: &Path) -> Result<(), String> {
    if !config_path.exists() {
        return Err(format!("Configuration file not found at {:?}", config_path));
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read configuration file: {}", e))?;

    let settings: LiemBarSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Configuration schema validation failed: {}", e))?;

    // 1. Check active profile exists
    if !settings.profiles.contains_key(&settings.active_profile) {
        return Err(format!(
            "Active profile '{}' is not defined in profiles list",
            settings.active_profile
        ));
    }

    // 2. Check each profile's layouts exist
    for (name, profile) in &settings.profiles {
        for bar in &profile.bars {
            if !settings.layouts.contains_key(&bar.layout_name) {
                return Err(format!(
                    "Profile '{}' references layout '{}' which is not defined in layouts list",
                    name, bar.layout_name
                ));
            }
        }
    }

    println!("Configuration is valid!");
    Ok(())
}

/// Query and display hardware diagnostics.
pub fn run_diagnostics(config_path: &Path) -> Result<(), String> {
    println!("=== LIEM BAR DIAGNOSTICS ===");

    // 1. Check Configuration
    print!("Checking configuration: ");
    match validate_config(config_path) {
        Ok(_) => println!("OK"),
        Err(e) => println!("FAILED: {}", e),
    }

    // 2. Check Monitors
    println!("\nChecking Displays:");
    let monitors = enumerate_monitors();
    println!("  Total connected monitors: {}", monitors.len());
    for (i, m) in monitors.iter().enumerate() {
        let w = m.bounds.right - m.bounds.left;
        let h = m.bounds.bottom - m.bounds.top;
        println!(
            "  - Display #{}: {} ({}x{}) Primary: {}",
            i + 1,
            m.name.trim(),
            w,
            h,
            m.is_primary
        );
    }

    // 3. Check Battery
    println!("\nChecking Power System:");
    let battery_service = Win32BatteryService::new();
    if let Ok((percent, is_ac, is_saver)) = battery_service.get_battery_info() {
        println!("  AC Connected: {}", is_ac);
        println!("  Battery Level: {}%", percent);
        println!("  Battery Saver Active: {}", is_saver);
    } else {
        println!("  Failed to query battery diagnostics");
    }

    println!("=============================");
    Ok(())
}

/// Interactively switch the active layout profile.
pub fn edit_active_profile(config_path: &Path) -> Result<(), String> {
    if !config_path.exists() {
        return Err(format!("Configuration file not found at {:?}", config_path));
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read configuration: {}", e))?;

    let mut settings: LiemBarSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Schema validation failed: {}", e))?;

    println!("Current active profile: {}", settings.active_profile);
    println!("Available profiles:");
    let profiles: Vec<String> = settings.profiles.keys().cloned().collect();
    for (i, name) in profiles.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }

    print!("\nEnter profile number or name to switch: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read input: {}", e))?;
    let input = input.trim();

    if input.is_empty() {
        return Err("No profile selected".to_string());
    }

    let target_profile = if let Ok(idx) = input.parse::<usize>() {
        if idx > 0 && idx <= profiles.len() {
            profiles[idx - 1].clone()
        } else {
            return Err("Invalid profile index".to_string());
        }
    } else if settings.profiles.contains_key(input) {
        input.to_string()
    } else {
        return Err(format!("Profile '{}' not found", input));
    };

    settings.active_profile = target_profile.clone();

    let updated = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Serialization failed: {}", e))?;

    fs::write(config_path, updated)
        .map_err(|e| format!("Failed to save configuration: {}", e))?;

    println!("Successfully switched active profile to '{}'!", target_profile);
    Ok(())
}
