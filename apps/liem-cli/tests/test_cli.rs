use std::process::Command;

#[test]
fn test_liem_help_routing() {
    // Locate target debug build of liem binary
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop(); // Remove target/debug/deps/...
    if exe_path.ends_with("deps") {
        exe_path.pop();
    }
    let liem_bin = exe_path.join("liem.exe");

    if liem_bin.exists() {
        let output = Command::new(&liem_bin)
            .arg("help")
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Liem Desktop Ecosystem CLI"));
        assert!(stdout.contains("wallpaper"));
        assert!(stdout.contains("bar"));
    }
}
