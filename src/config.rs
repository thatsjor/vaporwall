use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use ini::Ini;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaporConfig {
    pub core: CoreConfig,
    pub monitor: MonitorConfig,
    pub performance: PerformanceConfig,
    pub colors: ColorsConfig,
    pub wall_specific: WallSpecificConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub current_wall: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub mode: String,
    pub displays: String,
    pub center_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub fps_limit: u32,
    pub resolution_scale: f32,
    pub enable_shadows: bool,
    pub enable_antialiasing: bool,
    pub detail_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorsConfig {
    pub use_pywal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallSpecificConfig {
    pub speed_multiplier: f32,
}

impl Default for VaporConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig {
                current_wall: "example.html".to_string(),
            },
            monitor: MonitorConfig {
                mode: "span".to_string(),
                displays: "all".to_string(),
                center_display: "auto".to_string(),
            },
            performance: PerformanceConfig {
                fps_limit: 30,
                resolution_scale: 0.5,
                enable_shadows: false,
                enable_antialiasing: false,
                detail_level: 0.5,
            },
            colors: ColorsConfig {
                use_pywal: true,
            },
            wall_specific: WallSpecificConfig {
                speed_multiplier: 0.5,
            },
        }
    }
}

impl VaporConfig {
    pub fn config_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/"));
        PathBuf::from(home).join(".config").join("vaporwall")
    }

    pub fn walls_dir() -> PathBuf {
        Self::config_dir().join("walls")
    }

    pub fn config_file_path() -> PathBuf {
        Self::config_dir().join("vapor.conf")
    }

    pub fn load_or_create() -> Result<Self> {
        let config_dir = Self::config_dir();
        let walls_dir = Self::walls_dir();
        let config_file = Self::config_file_path();

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        }
        
        if !walls_dir.exists() {
            fs::create_dir_all(&walls_dir).context("Failed to create walls directory")?;
        }

        if !config_file.exists() {
            let default_config = Self::default();
            default_config.save(&config_file).context("Failed to save default config")?;
            return Ok(default_config);
        }

        Self::load_from_file(&config_file)
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        let mut config = Self::default();
        
        if let Ok(ini) = Ini::load_from_file(path) {
            if let Some(core) = ini.section(Some("core")) {
                if let Some(wall) = core.get("current_wall") {
                    config.core.current_wall = wall.to_string();
                }
            }

            if let Some(mon) = ini.section(Some("monitor")) {
                if let Some(mode) = mon.get("mode") {
                    config.monitor.mode = mode.to_string();
                }
                if let Some(displays) = mon.get("displays") {
                    config.monitor.displays = displays.to_string();
                }
                if let Some(center) = mon.get("center_display") {
                    config.monitor.center_display = center.to_string();
                }
            }

            if let Some(perf) = ini.section(Some("performance")) {
                if let Some(fps) = perf.get("fps_limit") {
                    config.performance.fps_limit = fps.parse().unwrap_or(config.performance.fps_limit);
                }
                if let Some(scale) = perf.get("resolution_scale") {
                    config.performance.resolution_scale = scale.parse().unwrap_or(config.performance.resolution_scale);
                }
                if let Some(shadows) = perf.get("enable_shadows") {
                    config.performance.enable_shadows = shadows.parse().unwrap_or(config.performance.enable_shadows);
                }
                if let Some(aa) = perf.get("enable_antialiasing") {
                    config.performance.enable_antialiasing = aa.parse().unwrap_or(config.performance.enable_antialiasing);
                }
                if let Some(detail) = perf.get("detail_level") {
                    config.performance.detail_level = detail.parse().unwrap_or(config.performance.detail_level);
                }
            }

            if let Some(colors) = ini.section(Some("colors")) {
                if let Some(use_pywal) = colors.get("use_pywal") {
                    config.colors.use_pywal = use_pywal.parse().unwrap_or(config.colors.use_pywal);
                }
            }

            if let Some(wall_spec) = ini.section(Some("wall_specific")) {
                if let Some(speed) = wall_spec.get("speed_multiplier") {
                    config.wall_specific.speed_multiplier = speed.parse().unwrap_or(config.wall_specific.speed_multiplier);
                }
            }
        }

        Ok(config)
    }

    fn save(&self, path: &Path) -> Result<()> {
        let mut conf = Ini::new();
        
        conf.with_section(Some("core"))
            .set("current_wall", &self.core.current_wall);
            
        conf.with_section(Some("monitor"))
            .set("mode", &self.monitor.mode)
            .set("displays", &self.monitor.displays)
            .set("center_display", &self.monitor.center_display);
            
        conf.with_section(Some("performance"))
            .set("fps_limit", self.performance.fps_limit.to_string())
            .set("resolution_scale", self.performance.resolution_scale.to_string())
            .set("enable_shadows", self.performance.enable_shadows.to_string())
            .set("enable_antialiasing", self.performance.enable_antialiasing.to_string())
            .set("detail_level", self.performance.detail_level.to_string());
            
        conf.with_section(Some("colors"))
            .set("use_pywal", self.colors.use_pywal.to_string());
            
        conf.with_section(Some("wall_specific"))
            .set("speed_multiplier", self.wall_specific.speed_multiplier.to_string());
            
        conf.write_to_file(path).context("Failed to write ini file")?;
        
        Ok(())
    }
}
