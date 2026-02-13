use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub gemini_api_key: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        if let Some(path) = Self::get_path() {
            if let Ok(content) = fs::read_to_string(path) {
                return serde_json::from_str(&content).unwrap_or_default();
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(path) = Self::get_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string_pretty(self)?;
            fs::write(path, content)?;
        }
        Ok(())
    }

    fn get_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "arthema", "arthema")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
    }
}
