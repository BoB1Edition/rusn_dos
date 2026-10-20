// Ver: 1 File: ./crate/emu_config/src/lib.rs
use serde::{Deserialize, Serialize};

// ============================================================
// Разрешение окна хоста
// ============================================================
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Resolution {
    #[serde(default = "default_width")]
    pub width: u16,
    #[serde(default = "default_height")]
    pub height: u16,
}

fn default_width() -> u16 {
    320
}
fn default_height() -> u16 {
    240
}

impl Default for Resolution {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
        }
    }
}

// ============================================================
// Драйверы (диски)
// ============================================================
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DriverConfig {
    pub letter: String,
    pub path: String,
    #[serde(default = "default_true")]
    pub read_only: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            letter: "Z".to_string(),
            path: "./".to_string(),
            read_only: true,
        }
    }
}

// ============================================================
// Машина: A20, EMS, DPMI, блокировки
// ============================================================
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MachineConfig {
    #[serde(default)]
    pub a20_enable: bool,
    #[serde(default)]
    pub ems: bool,
    #[serde(default)]
    pub dpmi: bool,
    #[serde(default)]
    pub disable_change_flags: Vec<String>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            a20_enable: false,
            ems: false,
            dpmi: false,
            disable_change_flags: vec![],
        }
    }
}

impl MachineConfig {
    #[inline]
    pub fn is_feature_locked(&self, feature_name: &str) -> bool {
        self.disable_change_flags.iter().any(|f| f == feature_name)
    }
}

// ============================================================
// Видео
// ============================================================
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoConfig {
    #[serde(default = "default_video_type")]
    pub r#type: String,
    #[serde(default = "default_available_video_types")]
    pub available_types: Vec<String>,
}

fn default_video_type() -> String {
    "vga".to_string()
}
fn default_available_video_types() -> Vec<String> {
    let ret = vec!["vga".to_string(), "svga".to_string()];
    ret
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            r#type: default_video_type(),
            available_types: default_available_video_types(),
        }
    }
}

// ============================================================
// Звук
// ============================================================
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoundConfig {
    #[serde(default = "default_sound_type")]
    pub r#type: String,
    #[serde(default = "default_available_sound_types")]
    pub available_types: Vec<String>,
}

fn default_sound_type() -> String {
    "pc_speaker".to_string()
}
fn default_available_sound_types() -> Vec<String> {
    let ret = vec!["pc_speaker".to_string(), "sb16".to_string()];
    ret
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            r#type: default_sound_type(),
            available_types: default_available_sound_types(),
        }
    }
}

// ============================================================
// CPU (задел на будущее)
// ============================================================
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CpuConfig {
    #[serde(default = "default_cpu_name")]
    pub name: String,
    #[serde(default = "default_cpu_type")]
    pub r#type: String,
    #[serde(default = "default_cpu_features")]
    pub features: Vec<String>,
}

fn default_cpu_name() -> String {
    "x86".to_string()
}
fn default_cpu_type() -> String {
    "x86".to_string()
}
fn default_cpu_features() -> Vec<String> {
    let ret = vec!["default".to_string()];
    ret
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            name: default_cpu_name(),
            r#type: default_cpu_type(),
            features: default_cpu_features(),
        }
    }
}
