// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling configuration

use super::shortcuts::Shortcuts;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[cfg(feature = "yaml")]
    #[error("config (de)serialisation to YAML failed")]
    Yaml(#[from] serde_yaml::Error),
    #[cfg(feature = "json")]
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
    #[cfg_attr(feature = "serde", serde(default = "Shortcuts::platform_defaults"))]
    pub shortcuts: Shortcuts,
}

impl Default for Config {
    fn default() -> Self {
        let shortcuts = Shortcuts::platform_defaults();
        Config { shortcuts }
    }
}

impl Config {
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
