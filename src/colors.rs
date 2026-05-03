use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PywalColors {
    pub special: SpecialColors,
    pub colors: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialColors {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
}

impl PywalColors {
    pub fn load() -> Result<Self> {
        let home = std::env::var("HOME").context("HOME not set")?;
        let path = PathBuf::from(home).join(".cache/wal/colors.json");
        let content = fs::read_to_string(path).context("Could not read pywal colors.json")?;
        let colors: PywalColors = serde_json::from_str(&content).context("Could not parse pywal colors.json")?;
        Ok(colors)
    }

    pub fn get_gl_color(&self, name: &str) -> [f32; 3] {
        let hex = if name == "background" {
            &self.special.background
        } else if name == "foreground" {
            &self.special.foreground
        } else {
            self.colors.get(name).map(|s| s.as_str()).unwrap_or("#000000")
        };

        hex_to_rgb(hex)
    }
}

fn hex_to_rgb(hex: &str) -> [f32; 3] {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return [0.0, 0.0, 0.0];
    }
    
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    
    [r, g, b]
}
