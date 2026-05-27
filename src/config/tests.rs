use super::*;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.settings.deadzone_pixels, 8);
    assert_eq!(config.settings.snapping_threshold_pixels, 10);
    assert_eq!(config.settings.layouts_enabled, Some(true));
    assert_eq!(config.settings.gestures_enabled, Some(true));
    assert!(config.is_blacklisted("cs2.exe"));
    assert!(config.is_blacklisted("CS2.EXE"));
    assert!(!config.is_blacklisted("notepad.exe"));

    let ui = config.get_ui_config();
    assert_eq!(ui.preview_fill_color, 0x00B98029);
    assert_eq!(ui.preview_border_color, 0x00DB9834);
    assert_eq!(ui.preview_opacity, 120);
    assert_eq!(ui.preview_border_radius, 8);
    assert_eq!(ui.gap_pixels, 8);
}

#[test]
fn test_parse_config_toml() {
    let content = r#"
        [settings]
        deadzone_pixels = 8
        snapping_threshold_pixels = 10
        layouts_enabled = false
        gestures_enabled = false

        [blacklist]
        processes = ["cs2.exe"]

        [ui]
        preview_fill_color = 0x00B98029
        preview_border_color = 0x00DB9834
        preview_opacity = 120
        preview_border_radius = 8
        gap_pixels = 8
    "#;
    let parsed = toml::from_str::<Config>(content);
    assert!(parsed.is_ok(), "Failed to parse TOML: {:?}", parsed.err());
    let config = parsed.unwrap();
    assert_eq!(config.settings.layouts_enabled, Some(false));
    assert_eq!(config.settings.gestures_enabled, Some(false));
}
