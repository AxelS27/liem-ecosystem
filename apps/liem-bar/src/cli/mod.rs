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
    let current_version = env!("CARGO_PKG_VERSION");
    let repo = "AxelS27/liem-ecosystem";
    let asset_name = "LiemBarSetup.exe";

    println!("Checking for Liem Bar updates from GitHub (AxelS27/liem-ecosystem)...");

    let ps_script = format!(
        "$version = '{}'; \
         $repo = '{}'; \
         $asset_name = '{}'; \
         try {{ \
             $r = Invoke-RestMethod -Uri \"https://api.github.com/repos/$repo/releases/latest\" -UserAgent \"LiemBar\" -ErrorAction Stop; \
             $latest = $r.tag_name.TrimStart('v'); \
             if ($latest -ne $version) {{ \
                 Write-Output \"NEW_VERSION:$latest\"; \
                 $asset = $r.assets | Where-Object {{ $_.name -eq $asset_name }} | Select-Object -First 1; \
                 if ($asset) {{ \
                     Write-Output \"DOWNLOADING:$($asset.name)\"; \
                     $tempPath = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), $asset.name); \
                     Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tempPath -UserAgent \"LiemBar\" -ErrorAction Stop; \
                     Write-Output \"INSTALLING\"; \
                     Start-Process -FilePath $tempPath -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART'; \
                     Write-Output \"SUCCESS\"; \
                 }} else {{ \
                     Write-Output \"ERROR:No installer asset found in the latest release ($asset_name)\"; \
                 }} \
             }} else {{ \
                 Write-Output \"UPTODATE\"; \
             }} \
         }} catch {{ \
             Write-Output \"ERROR:$($_.Exception.Message)\"; \
         }}",
        current_version, repo, asset_name
    );

    let output = std::process::Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut is_downloading = false;

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("NEW_VERSION:") {
            let latest_version = line.trim_start_matches("NEW_VERSION:");
            println!("New version v{} is available!", latest_version);
        } else if line.starts_with("DOWNLOADING:") {
            let name = line.trim_start_matches("DOWNLOADING:");
            println!("Downloading latest installer ({}) to Temp folder...", name);
            is_downloading = true;
        } else if line == "INSTALLING" {
            println!("Launching silent installer in background...");
        } else if line == "SUCCESS" {
            println!("Update launched successfully! Liem Bar will restart shortly.");
            return Ok(());
        } else if line == "UPTODATE" {
            println!("Liem Bar is already up-to-date (v{}).", current_version);
            return Ok(());
        } else if line.starts_with("ERROR:") {
            return Err(line.trim_start_matches("ERROR:").to_string());
        }
    }

    if is_downloading {
        Err("Update process terminated unexpectedly during download.".to_string())
    } else {
        Err(format!("Failed to retrieve update status. Raw output: {stdout}"))
    }
}

/// Validate configuration integrity.
pub fn validate_config(config_path: &Path) -> Result<(), String> {
    if !config_path.exists() {
        return Err(format!("Configuration file not found at {:?}", config_path));
    }

    let (_, settings) = crate::core::config::load_or_create_bar_config(config_path)
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
            let layout_name = bar.layout_name.as_ref().unwrap_or(name);
            if !settings.layouts.contains_key(layout_name) {
                return Err(format!(
                    "Profile '{}' references layout '{}' which is not defined in layouts list",
                    name, layout_name
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
