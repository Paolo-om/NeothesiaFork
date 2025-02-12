use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[cfg(feature = "app")]
use crate::output_manager::OutputDescriptor;

#[derive(Serialize, Deserialize, Default)]
pub struct ColorSchema {
    pub base: (u8, u8, u8),
    pub dark: (u8, u8, u8),
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_speed_multiplier")]
    pub speed_multiplier: f32,

    #[serde(default = "default_playback_offset")]
    pub playback_offset: f32,

    #[serde(default = "default_play_along")]
    #[serde(skip_serializing)]
    pub play_along: bool,

    #[serde(default = "default_color_schema")]
    pub color_schema: Vec<ColorSchema>,

    #[serde(default)]
    pub background_color: (u8, u8, u8),

    #[serde(default = "default_output")]
    pub output: Option<String>,
    pub input: Option<String>,

    pub soundfont_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        let config: Option<Config> = if let Some(path) = crate::utils::resources::settings_ron() {
            if let Ok(file) = std::fs::read_to_string(path) {
                match ron::from_str(&file) {
                    Ok(config) => Some(config),
                    Err(err) => {
                        log::error!("{:#?}", err);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        config.unwrap_or_else(|| Self {
            speed_multiplier: default_speed_multiplier(),
            playback_offset: default_playback_offset(),
            play_along: default_play_along(),
            color_schema: default_color_schema(),
            background_color: Default::default(),
            output: default_output(),
            input: None,
            soundfont_path: None,
        })
    }

    #[cfg(feature = "app")]
    pub fn set_output(&mut self, v: &OutputDescriptor) {
        if let OutputDescriptor::DummyOutput = v {
            self.output = None;
        } else {
            self.output = Some(v.to_string());
        }
    }

    pub fn set_input<D: std::fmt::Display>(&mut self, v: Option<D>) {
        self.input = v.map(|v| v.to_string());
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        if let Ok(s) = ron::ser::to_string_pretty(self, Default::default()) {
            if let Some(path) = crate::utils::resources::settings_ron() {
                std::fs::create_dir_all(path.parent().unwrap()).ok();
                std::fs::write(path, &s).ok();
            }
        }
    }
}

fn default_speed_multiplier() -> f32 {
    1.0
}

fn default_playback_offset() -> f32 {
    0.0
}

fn default_play_along() -> bool {
    false
}

fn default_color_schema() -> Vec<ColorSchema> {
    vec![
        ColorSchema {
            base: (93, 188, 255),
            dark: (48, 124, 255),
        },
        ColorSchema {
            base: (210, 89, 222),
            dark: (125, 69, 134),
        },
    ]
}

fn default_output() -> Option<String> {
    Some("Buildin Synth".into())
}
