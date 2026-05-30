use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub deadzone_pixels: i32,
    pub snapping_threshold_pixels: i32,
    pub layouts_enabled: Option<bool>,
    pub gestures_enabled: Option<bool>,
    pub split_zones_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Blacklist {
    pub processes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub preview_fill_color: u32,
    pub preview_border_color: u32,
    pub preview_opacity: u8,
    pub preview_border_radius: i32,
    pub gap_pixels: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub settings: Settings,
    pub blacklist: Blacklist,
    pub ui: Option<UiConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            settings: Settings {
                deadzone_pixels: 8,
                snapping_threshold_pixels: 10,
                layouts_enabled: Some(true),
                gestures_enabled: Some(true),
                split_zones_enabled: Some(true),
            },
            blacklist: Blacklist {
                processes: vec![
                    "cs2.exe".to_string(),
                    "VALORANT-Win64-Shipping.exe".to_string(),
                    "Overwatch.exe".to_string(),
                    "League of Legends.exe".to_string(),
                ],
            },
            ui: Some(UiConfig {
                preview_fill_color: 0x00B98029,
                preview_border_color: 0x00DB9834,
                preview_opacity: 120,
                preview_border_radius: 8,
                gap_pixels: 8,
            }),
        }
    }
}
