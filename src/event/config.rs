// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling configuration

use super::shortcuts::Shortcuts;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
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
    /// TOML
    #[error("TOML")]
    Toml,
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
pub struct Config {
    pub shortcuts: Shortcuts,
}

impl Default for Config {
    fn default() -> Self {
        let mut shortcuts = Shortcuts::new();
        shortcuts.load_platform_defaults();
        Config { shortcuts }
    }
}

impl Config {
    fn guess_format(path: &Path) -> ConfigFormat {
        // use == since there is no OsStr literal
        if let Some(ext) = path.extension() {
            if ext == "toml" {
                ConfigFormat::Toml
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
            _ => Err(ConfigError::UnsupportedFormat(format)),
        }
    }

    /// Write to a path
    pub fn write_path(&self, path: &Path, mut format: ConfigFormat) -> Result<(), ConfigError> {
        if format == ConfigFormat::None {
            format = Self::guess_format(path);
        }

        match format {
            _ => Err(ConfigError::UnsupportedFormat(format)),
        }
    }
}
