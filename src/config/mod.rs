pub mod types;
pub mod atomics;
pub mod bypass;
pub mod registry;

pub use types::*;
pub use atomics::*;

use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

fn load_config() -> Config {
    let mut resolved_path = None;

    // 1. Check next to the executable (and parent folders up to 3 levels)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let path = exe_dir.join("config.toml");
            if path.exists() {
                resolved_path = Some(path);
            } else {
                // If in target/release/ or target/debug/, search up to parent folders (e.g. project root)
                let mut current = exe_dir.to_path_buf();
                for _ in 0..3 {
                    if let Some(parent) = current.parent() {
                        let path = parent.join("config.toml");
                        if path.exists() {
                            resolved_path = Some(path);
                            break;
                        }
                        current = parent.to_path_buf();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    // 2. Check current working directory
    if resolved_path.is_none() {
        let cwd_path = Path::new("config.toml");
        if cwd_path.exists() {
            resolved_path = Some(cwd_path.to_path_buf());
        }
    }

    if let Some(config_path) = resolved_path {
        match fs::read_to_string(&config_path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(parsed) => {
                    println!(
                        "[OK] Loaded config.toml successfully from {:?}",
                        config_path
                    );
                    return parsed;
                }
                Err(e) => {
                    eprintln!(
                        "[WARN] Failed to parse config.toml at {:?}: {}. Falling back to defaults.",
                        config_path, e
                    );
                }
            },
            Err(e) => {
                eprintln!(
                    "[WARN] Failed to read config.toml at {:?}: {}. Falling back to defaults.",
                    config_path, e
                );
            }
        }
    } else {
        println!("[INFO] config.toml not found in any search path. Using defaults.");
    }
    Config::default()
}

// Global thread-safe static CONFIG lazy cell
pub static CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| {
    let cfg = load_config();
    sync_atomics(&cfg);
    Mutex::new(cfg)
});

/// Dynamic hot-reloading function to read config.toml from disk on demand.
pub fn reload_config() {
    if let Ok(mut guard) = CONFIG.lock() {
        let cfg = load_config();
        sync_atomics(&cfg);
        *guard = cfg;
    }
}

impl Config {
    /// Safe helper that returns custom UI colors/opacity if defined, or falls back to standard slate-blue.
    pub fn get_ui_config(&self) -> UiConfig {
        self.ui.clone().unwrap_or(UiConfig {
            preview_fill_color: 0x00B98029,
            preview_border_color: 0x00DB9834,
            preview_opacity: 120,
            preview_border_radius: 8,
            gap_pixels: 8,
        })
    }

    /// Returns true if the process name (case-insensitive) matches any entry in the blacklist.
    pub fn is_blacklisted(&self, process_name: &str) -> bool {
        let process_name_lower = process_name.to_lowercase();
        self.blacklist
            .processes
            .iter()
            .any(|p| p.to_lowercase() == process_name_lower)
    }
}

#[cfg(test)]
mod tests;
