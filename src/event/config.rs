// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling configuration

use super::{shortcuts::Shortcuts, ModifiersState};
use crate::cast::Cast;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[cfg(feature = "yaml")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "yaml")))]
    #[error("config (de)serialisation to YAML failed")]
    Yaml(#[from] serde_yaml::Error),
    #[cfg(feature = "json")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "json")))]
    #[error("config (de)serialisation to JSON failed")]
    Json(#[from] serde_json::Error),
    #[error("error reading / writing config file")]
    IoError(#[from] std::io::Error),
    #[error("format not supported: {0}")]
    UnsupportedFormat(ConfigFormat),
}

/// Serialisation formats
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Error)]
pub enum ConfigFormat {
    /// Not specified: guess from the path
    #[error("no format")]
    None,
    /// JSON
    #[error("JSON")]
    Json,
    /// TOML
    #[error("TOML")]
    Toml,
    /// YAML
    #[error("YAML")]
    Yaml,
    /// Error: unable to guess format
    #[error("(unknown format)")]
    Unknown,
}

impl Default for ConfigFormat {
    fn default() -> Self {
        ConfigFormat::None
    }
}

/// Event handling configuration
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Delay before opening/closing menus on mouse hover
    #[cfg_attr(feature = "serde", serde(default = "defaults::menu_delay_ns"))]
    pub menu_delay_ns: u32,

    /// Delay before switching from panning to text-selection mode
    #[cfg_attr(
        feature = "serde",
        serde(default = "defaults::touch_text_sel_delay_ns")
    )]
    pub touch_text_sel_delay_ns: u32,

    /// Drag distance threshold before panning (scrolling) starts
    ///
    /// When the distance moved is greater than this threshold, panning should
    /// start; otherwise the system should wait for the text-selection timer.
    /// We currently recommend the L-inf distance metric (max of abs of values).
    // TODO: multiply by scale factor on access?
    #[cfg_attr(feature = "serde", serde(default = "defaults::pan_dist_thresh"))]
    pub pan_dist_thresh: i32,

    /// When to pan general widgets (unhandled events) with the mouse
    #[cfg_attr(feature = "serde", serde(default = "defaults::mouse_pan"))]
    pub mouse_pan: MousePan,
    /// When to pan text fields with the mouse
    #[cfg_attr(feature = "serde", serde(default = "defaults::mouse_text_pan"))]
    pub mouse_text_pan: MousePan,

    #[cfg_attr(feature = "serde", serde(default = "Shortcuts::platform_defaults"))]
    pub shortcuts: Shortcuts,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            menu_delay_ns: defaults::menu_delay_ns(),
            touch_text_sel_delay_ns: defaults::touch_text_sel_delay_ns(),
            pan_dist_thresh: defaults::pan_dist_thresh(),
            mouse_pan: defaults::mouse_pan(),
            mouse_text_pan: defaults::mouse_text_pan(),
            shortcuts: Shortcuts::platform_defaults(),
        }
    }
}

impl Config {
    /// Get menu delay as a `Duration`
    pub fn menu_delay(&self) -> Duration {
        Duration::from_nanos(self.menu_delay_ns.cast())
    }

    /// Get touch selection delay as a `Duration`
    pub fn touch_text_sel_delay(&self) -> Duration {
        Duration::from_nanos(self.touch_text_sel_delay_ns.cast())
    }

    fn guess_format(path: &Path) -> ConfigFormat {
        // use == since there is no OsStr literal
        if let Some(ext) = path.extension() {
            if ext == "json" {
                ConfigFormat::Json
            } else if ext == "toml" {
                ConfigFormat::Toml
            } else if ext == "yaml" {
                ConfigFormat::Yaml
            } else {
                ConfigFormat::Unknown
            }
        } else {
            ConfigFormat::Unknown
        }
    }

    /// Read from a path
    pub fn from_path(path: &Path, mut format: ConfigFormat) -> Result<Self, ConfigError> {
        if format == ConfigFormat::None {
            format = Self::guess_format(path);
        }

        match format {
            #[cfg(feature = "json")]
            ConfigFormat::Json => {
                let r = std::io::BufReader::new(std::fs::File::open(path)?);
                Ok(serde_json::from_reader(r)?)
            }
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => {
                let r = std::io::BufReader::new(std::fs::File::open(path)?);
                Ok(serde_yaml::from_reader(r)?)
            }
            _ => Err(ConfigError::UnsupportedFormat(format)),
        }
    }

    /// Write to a path
    pub fn write_path(&self, path: &Path, mut format: ConfigFormat) -> Result<(), ConfigError> {
        if format == ConfigFormat::None {
            format = Self::guess_format(path);
        }

        match format {
            #[cfg(feature = "json")]
            ConfigFormat::Json => {
                let w = std::io::BufWriter::new(std::fs::File::create(path)?);
                serde_json::to_writer_pretty(w, self)?;
                Ok(())
            }
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => {
                let w = std::io::BufWriter::new(std::fs::File::create(path)?);
                serde_yaml::to_writer(w, self)?;
                Ok(())
            }
            // NOTE: Toml is not supported since the `toml` crate does not support enums as map keys
            _ => Err(ConfigError::UnsupportedFormat(format)),
        }
    }
}

/// When mouse-panning is enabled (click+drag to scroll)
///
/// For *text* objects, this may conflict with text selection, hence it is
/// recommended to require a modifier or disable this feature.
///
/// For non-text cases, this does not conflict with other event handlers since
/// panning is only possible when events are otherwise unused, thus `Always` is
/// acceptable (equivalent to touch scrolling).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MousePan {
    /// Disable
    Never,
    /// Only enable when the Alt key is held
    WithAlt,
    /// Only enable when the Ctrl key is held
    WithCtrl,
    /// Always enabled
    Always,
}

impl MousePan {
    /// Is this enabled with the current modifiers?
    pub fn is_enabled_with(self, modifiers: ModifiersState) -> bool {
        match self {
            MousePan::Never => false,
            MousePan::WithAlt => modifiers.alt(),
            MousePan::WithCtrl => modifiers.ctrl(),
            MousePan::Always => true,
        }
    }
}

mod defaults {
    use super::MousePan;

    pub fn menu_delay_ns() -> u32 {
        250_000_000
    }
    pub fn touch_text_sel_delay_ns() -> u32 {
        1_000_000_000
    }
    pub fn pan_dist_thresh() -> i32 {
        2
    }
    pub fn mouse_pan() -> MousePan {
        MousePan::Always
    }
    pub fn mouse_text_pan() -> MousePan {
        #[cfg(target_os = "windows")]
        {
            MousePan::WithAlt
        }
        #[cfg(not(target_os = "windows"))]
        {
            MousePan::WithCtrl
        }
    }
}
