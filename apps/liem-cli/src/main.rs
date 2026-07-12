use std::path::PathBuf;
use std::process::Command;

fn print_help() {
    println!("Liem Desktop Ecosystem CLI");
    println!("Usage: liem <command> [args...]");
    println!();
    println!("Commands:");
    println!("  wallpaper [args...]   Proxy command to Liem Wallpaper CLI (lw.exe)");
    println!("  bar [args...]         Proxy command to Liem Bar CLI (liem-bar.exe)");
    println!("  status                Show running status of ecosystem services");
    println!("  start                 Start all ecosystem services");
    println!("  stop                  Stop all ecosystem services");
    println!("  update                Check and perform ecosystem updates");
    println!("  help                  Show this help menu");
}

fn is_process_running(exe_name: &str) -> bool {
    if let Ok(output) = Command::new("tasklist.exe")
        .args(&["/FI", &format!("IMAGENAME eq {}", exe_name)])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains(exe_name)
    } else {
        false
    }
}

fn stop_process(exe_name: &str) {
    let _ = Command::new("taskkill.exe")
        .args(&["/F", "/IM", exe_name])
        .output();
}

fn start_process(exe_name: &str) -> Result<(), String> {
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    
    let mut target = exe_path.join(exe_name);
    if !target.exists() {
        if exe_name == "lw-service.exe" {
            target = exe_path.join("Liem Wallpaper").join("lw-service.exe");
        } else if exe_name == "liem-bar.exe" {
            target = exe_path.join("Liem Bar").join("liem-bar.exe");
        }
    }
    
    if !target.exists() {
        target = exe_path.join(exe_name);
    }
    
    if !target.exists() {
        return Err(format!("Could not locate executable: {}", exe_name));
    }
    
    Command::new(target)
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", exe_name, e))?;
        
    Ok(())
}

fn run_sub_binary(bin_name: &str, args: &[String]) -> Result<(), String> {
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    
    let mut target = exe_path.join(bin_name);
    if !target.exists() {
        if bin_name == "lw.exe" {
            target = exe_path.join("Liem Wallpaper").join("lw.exe");
        } else if bin_name == "lb.exe" {
            target = exe_path.join("Liem Bar").join("lb.exe");
        }
    }
    if !target.exists() {
        target = PathBuf::from(bin_name);
    }
    
    let mut child = Command::new(target)
        .args(args)
        .spawn()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
        
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn run_ecosystem_update() -> Result<(), String> {
    let current_version = env!("CARGO_PKG_VERSION");
    let repo = "AxelS27/liem-ecosystem";
    let asset_name = "LiemEcosystemSetup.exe";

    println!("Checking for Liem Desktop Ecosystem updates from GitHub (AxelS27/liem-ecosystem)...");

    let ps_script = format!(
        "$version = '{}'; \
         $repo = '{}'; \
         $asset_name = '{}'; \
         try {{ \
             $r = Invoke-RestMethod -Uri \"https://api.github.com/repos/$repo/releases/latest\" -UserAgent \"LiemEcosystem\" -ErrorAction Stop; \
             $latest = $r.tag_name.TrimStart('v'); \
             if ($latest -ne $version) {{ \
                 Write-Output \"NEW_VERSION:$latest\"; \
                 $asset = $r.assets | Where-Object {{ $_.name -eq $asset_name }} | Select-Object -First 1; \
                 if ($asset) {{ \
                     Write-Output \"DOWNLOADING:$($asset.name)\"; \
                     $tempPath = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), $asset.name); \
                     Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tempPath -UserAgent \"LiemEcosystem\" -ErrorAction Stop; \
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
            println!("Update launched successfully! The ecosystem will restart shortly.");
            return Ok(());
        } else if line == "UPTODATE" {
            println!("Liem Desktop Ecosystem is already up-to-date (v{}).", current_version);
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }
    
    let command = args[1].as_str();
    let sub_args = &args[2..];
    
    match command {
        "help" | "-h" | "--help" => {
            print_help();
        }
        "wallpaper" => {
            if let Err(e) = run_sub_binary("lw.exe", sub_args) {
                eprintln!("Error running Liem Wallpaper: {}", e);
                std::process::exit(1);
            }
        }
        "bar" => {
            if let Err(e) = run_sub_binary("lb.exe", sub_args) {
                eprintln!("Error running Liem Bar: {}", e);
                std::process::exit(1);
            }
        }
        "status" => {
            let wallpaper_running = is_process_running("lw-service.exe");
            let bar_running = is_process_running("liem-bar.exe");
            
            println!("Liem Desktop Services Status:");
            println!("  Liem Wallpaper Daemon (lw-service.exe): {}", if wallpaper_running { "RUNNING" } else { "STOPPED" });
            println!("  Liem Bar GUI (liem-bar.exe):           {}", if bar_running { "RUNNING" } else { "STOPPED" });
        }
        "start" => {
            println!("Starting Liem Desktop Ecosystem...");
            
            if !is_process_running("lw-service.exe") {
                match start_process("lw-service.exe") {
                    Ok(_) => println!("  Started Liem Wallpaper Daemon."),
                    Err(e) => eprintln!("  Error starting Liem Wallpaper Daemon: {}", e),
                }
            } else {
                println!("  Liem Wallpaper Daemon is already running.");
            }
            
            if !is_process_running("liem-bar.exe") {
                match start_process("liem-bar.exe") {
                    Ok(_) => println!("  Started Liem Bar GUI."),
                    Err(e) => eprintln!("  Error starting Liem Bar GUI: {}", e),
                }
            } else {
                println!("  Liem Bar GUI is already running.");
            }
        }
        "stop" => {
            println!("Stopping Liem Desktop Ecosystem...");
            stop_process("lw-service.exe");
            stop_process("liem-bar.exe");
            println!("  Stopped all services.");
        }
        "update" => {
            if let Err(e) = run_ecosystem_update() {
                eprintln!("Update failed: {}", e);
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("Unknown command: '{}'", other);
            println!();
            print_help();
            std::process::exit(1);
        }
    }
}
