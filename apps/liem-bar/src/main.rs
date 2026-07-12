use std::path::PathBuf;
use liem_bar::app::lifecycle::LiemBarApp;

fn get_config_path() -> PathBuf {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop(); // Get directory
        exe_path.push("config.json");
        exe_path
    } else {
        PathBuf::from("config.json")
    }
}

#[tokio::main]
async fn main() {
    std::env::set_var("SLINT_BACKEND", "winit");
    let config_path = get_config_path();
    let args: Vec<String> = std::env::args().collect();

    // If arguments are provided, route to CLI command runner and exit
    if args.len() > 1 {
        let cli_args = &args[1..];
        match liem_bar::cli::run_cli_command(cli_args, &config_path) {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    println!("Starting Liem Bar GUI with config: {:?}", config_path);
    let mut app = LiemBarApp::new(config_path);

    if let Err(e) = app.bootstrap() {
        eprintln!("Bootstrap failed: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = app.run() {
        eprintln!("Application runtime error: {}", e);
        std::process::exit(1);
    }

    println!("Liem Bar shut down cleanly.");
}
