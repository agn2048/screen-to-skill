use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub provider: String,
    pub models: HashMap<String, String>,
    pub base_urls: HashMap<String, String>,
    pub transcriber: String,
    pub whisper_model: String,
    pub language: String,
    pub sensitivity: f64,
    pub max_frames: u32,
    pub mic: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            models: HashMap::new(),
            base_urls: HashMap::new(),
            transcriber: "auto".into(),
            whisper_model: "base".into(),
            language: "auto".into(),
            sensitivity: 0.03,
            max_frames: 40,
            mic: true,
        }
    }
}

impl Settings {
    pub fn model(&self) -> &str {
        self.models.get(&self.provider).map(|s| s.trim()).unwrap_or("")
    }
    pub fn base_url(&self) -> &str {
        self.base_urls.get(&self.provider).map(|s| s.trim()).unwrap_or("")
    }
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_config_dir().map_err(|e| e.to_string())?.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    path(app)
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    load(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let p = path(&app)?;
    std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &p)).map_err(|e| e.to_string())
}
