use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    
    // Locate the main liem-bar.exe binary in the same directory
    let mut exe_path = std::env::current_exe().expect("Failed to get current executable path");
    exe_path.pop();
    
    let mut target = exe_path.join("liem-bar.exe");
    
    // Support installed folder structure fallback
    if !target.exists() {
        target = exe_path.join("Liem Bar").join("liem-bar.exe");
    }
    if !target.exists() {
        target = exe_path.join("liem-bar.exe");
    }
    
    if !target.exists() {
        eprintln!("Error: Could not locate liem-bar.exe adjacent to lb.exe");
        std::process::exit(1);
    }
    
    let mut child = Command::new(target)
        .args(&args)
        .spawn()
        .expect("Failed to start liem-bar");
        
    let status = child.wait().expect("Failed to wait on liem-bar process");
    std::process::exit(status.code().unwrap_or(1));
}
