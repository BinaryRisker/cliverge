use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub terminal_font_size: f32,
    pub auto_check_updates: bool,
    pub update_check_interval_minutes: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            terminal_font_size: 14.0,
            auto_check_updates: true,
            update_check_interval_minutes: 30,
        }
    }
}

impl AppSettings {
    /// Get the settings file path
    pub fn get_settings_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("cliverge");
        path.push("settings.json");
        path
    }
    
    /// Load settings from disk
    pub async fn load() -> Self {
        let path = Self::get_settings_path();
        
        if !path.exists() {
            return Self::default();
        }
        
        match fs::read_to_string(&path).await {
            Ok(contents) => {
                match serde_json::from_str::<AppSettings>(&contents) {
                    Ok(settings) => settings,
                    Err(e) => {
                        eprintln!("Failed to parse settings: {}", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read settings file: {}", e);
                Self::default()
            }
        }
    }
    
    /// Save settings to disk
    pub async fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::get_settings_path();
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json).await?;
        
        Ok(())
    }
    
    /// Apply theme to egui context
    pub fn apply_theme(&self, ctx: &egui::Context) {
        let visuals = match self.theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        };
        ctx.set_visuals(visuals);
    }
}
