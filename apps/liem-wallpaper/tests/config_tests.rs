use lw_core::{Config, EasingStyle, EasingDirection, WallpaperPosition};

#[test]
fn test_default_config_path() {
    let path = Config::default_path();
    assert!(path.to_string_lossy().contains("config.json"));
}

#[test]
fn test_config_lifecycle_states() {
    // Verify that configuration defaults are correctly validated
    let mut config = Config::default();
    config.wallpaper_dir = std::env::temp_dir();
    assert!(config.validate().is_ok());

    let temp_file = std::env::temp_dir().join("liem_wallpaper_lifecycle_test.json");
    
    // Test saving configuration
    assert!(config.save_to_file(&temp_file).is_ok());

    // Test loading configuration
    let loaded = Config::load_from_file(&temp_file).unwrap();
    assert_eq!(config.wallpaper_dir, loaded.wallpaper_dir);
    assert_eq!(config.position, loaded.position);

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}
