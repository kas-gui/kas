// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration read/write utilities

use crate::draw::DrawSharedImpl;
use crate::theme::{Theme, ThemeConfig};
#[cfg(feature = "serde")] use crate::util::warn_about_error;
#[cfg(feature = "serde")]
use serde::{de::DeserializeOwned, Serialize};
use std::env::var;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

/// Config mode
///
/// See [`Options::from_env`] documentation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConfigMode {
    /// Read-only mode
    Read,
    /// Read-write mode
    ///
    /// This mode reads config on start and writes changes on exit.
    ReadWrite,
    /// Use default config and write out
    ///
    /// This mode only writes initial (default) config and does not update.
    WriteDefault,
}

/// Configuration read/write/format errors
#[derive(Error, Debug)]
pub enum Error {
    #[cfg(feature = "yaml")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "yaml")))]
    #[error("config (de)serialisation to YAML failed")]
    Yaml(#[from] serde_yaml::Error),

    #[cfg(feature = "json")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "json")))]
    #[error("config (de)serialisation to JSON failed")]
    Json(#[from] serde_json::Error),

    #[cfg(feature = "ron")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "ron")))]
    #[error("config serialisation to RON failed")]
    Ron(#[from] ron::Error),

    #[cfg(feature = "ron")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "ron")))]
    #[error("config deserialisation from RON failed")]
    RonSpanned(#[from] ron::error::SpannedError),

    #[cfg(feature = "toml")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "toml")))]
    #[error("config deserialisation from TOML failed")]
    TomlDe(#[from] toml::de::Error),

    #[cfg(feature = "toml")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "toml")))]
    #[error("config serialisation to TOML failed")]
    TomlSer(#[from] toml::ser::Error),

    #[error("error reading / writing config file")]
    IoError(#[from] std::io::Error),

    #[error("format not supported: {0}")]
    UnsupportedFormat(Format),
}

/// Configuration serialisation formats
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Error)]
pub enum Format {
    /// Not specified: guess from the path
    #[default]
    #[error("no format")]
    None,

    /// JavaScript Object Notation
    #[error("JSON")]
    Json,

    /// Tom's Obvious Minimal Language
    #[error("TOML")]
    Toml,

    /// YAML Ain't Markup Language
    #[error("YAML")]
    Yaml,

    /// Rusty Object Notation
    #[error("RON")]
    Ron,

    /// Error: unable to guess format
    #[error("(unknown format)")]
    Unknown,
}

impl Format {
    /// Guess format from the path name
    ///
    /// This does not open the file.
    ///
    /// Potentially fallible: on error, returns [`Format::Unknown`].
    /// This may be due to unrecognised file extension or due to the required
    /// feature not being enabled.
    pub fn guess_from_path(path: &Path) -> Format {
        // use == since there is no OsStr literal
        if let Some(ext) = path.extension() {
            if ext == "json" {
                Format::Json
            } else if ext == "toml" {
                Format::Toml
            } else if ext == "yaml" {
                Format::Yaml
            } else if ext == "ron" {
                Format::Ron
            } else {
                Format::Unknown
            }
        } else {
            Format::Unknown
        }
    }

    /// Read from a path
    #[cfg(feature = "serde")]
    pub fn read_path<T: DeserializeOwned>(self, path: &Path) -> Result<T, Error> {
        log::info!("read_path: path={}, format={:?}", path.display(), self);
        match self {
            #[cfg(feature = "json")]
            Format::Json => {
                let r = std::io::BufReader::new(std::fs::File::open(path)?);
                Ok(serde_json::from_reader(r)?)
            }
            #[cfg(feature = "yaml")]
            Format::Yaml => {
                let r = std::io::BufReader::new(std::fs::File::open(path)?);
                Ok(serde_yaml::from_reader(r)?)
            }
            #[cfg(feature = "ron")]
            Format::Ron => {
                let r = std::io::BufReader::new(std::fs::File::open(path)?);
                Ok(ron::de::from_reader(r)?)
            }
            #[cfg(feature = "toml")]
            Format::Toml => {
                let contents = std::fs::read_to_string(path)?;
                Ok(toml::from_str(&contents)?)
            }
            _ => {
                let _ = path; // squelch unused warning
                Err(Error::UnsupportedFormat(self))
            }
        }
    }

    /// Write to a path
    #[cfg(feature = "serde")]
    pub fn write_path<T: Serialize>(self, path: &Path, value: &T) -> Result<(), Error> {
        log::info!("write_path: path={}, format={:?}", path.display(), self);
        // Note: we use to_string*, not to_writer*, since the latter may
        // generate incomplete documents on failure.
        match self {
            #[cfg(feature = "json")]
            Format::Json => {
                let text = serde_json::to_string_pretty(value)?;
                std::fs::write(path, &text)?;
                Ok(())
            }
            #[cfg(feature = "yaml")]
            Format::Yaml => {
                let text = serde_yaml::to_string(value)?;
                std::fs::write(path, text)?;
                Ok(())
            }
            #[cfg(feature = "ron")]
            Format::Ron => {
                let pretty = ron::ser::PrettyConfig::default();
                let text = ron::ser::to_string_pretty(value, pretty)?;
                std::fs::write(path, &text)?;
                Ok(())
            }
            #[cfg(feature = "toml")]
            Format::Toml => {
                let content = toml::to_string(value)?;
                std::fs::write(path, &content)?;
                Ok(())
            }
            _ => {
                let _ = (path, value); // squelch unused warnings
                Err(Error::UnsupportedFormat(self))
            }
        }
    }

    /// Guess format and load from a path
    #[cfg(feature = "serde")]
    #[inline]
    pub fn guess_and_read_path<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
        let format = Self::guess_from_path(path);
        format.read_path(path)
    }

    /// Guess format and write to a path
    #[cfg(feature = "serde")]
    #[inline]
    pub fn guess_and_write_path<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
        let format = Self::guess_from_path(path);
        format.write_path(path, value)
    }
}

/// Application configuration options
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Options {
    /// Config file path. Default: empty. See `KAS_CONFIG` doc.
    pub config_path: PathBuf,
    /// Theme config path. Default: empty.
    pub theme_config_path: PathBuf,
    /// Config mode. Default: Read.
    pub config_mode: ConfigMode,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            config_path: PathBuf::new(),
            theme_config_path: PathBuf::new(),
            config_mode: ConfigMode::Read,
        }
    }
}

impl Options {
    /// Construct a new instance, reading from environment variables
    ///
    /// The following environment variables are read, in case-insensitive mode.
    ///
    /// # Config files
    ///
    /// WARNING: file formats are not stable and may not be compatible across
    /// KAS versions (aside from patch versions)!
    ///
    /// The `KAS_CONFIG` variable, if given, provides a path to the KAS config
    /// file, which is read or written according to `KAS_CONFIG_MODE`.
    /// If `KAS_CONFIG` is not specified, platform-default configuration is used
    /// without reading or writing. This may change to use a platform-specific
    /// default path in future versions.
    ///
    /// The `KAS_THEME_CONFIG` variable, if given, provides a path to the theme
    /// config file, which is read or written according to `KAS_CONFIG_MODE`.
    /// If `KAS_THEME_CONFIG` is not specified, platform-default configuration
    /// is used without reading or writing. This may change to use a
    /// platform-specific default path in future versions.
    ///
    /// The `KAS_CONFIG_MODE` variable determines the read/write mode:
    ///
    /// -   `Read` (default): read-only
    /// -   `ReadWrite`: read on start-up, write on exit
    /// -   `WriteDefault`: generate platform-default configuration and write
    ///     it to the config path(s) specified, overwriting any existing config
    ///
    /// Note: in the future, the default will likely change to a read-write mode,
    /// allowing changes to be written out.
    pub fn from_env() -> Self {
        let mut options = Options::default();

        if let Ok(v) = var("KAS_CONFIG") {
            options.config_path = v.into();
        }

        if let Ok(v) = var("KAS_THEME_CONFIG") {
            options.theme_config_path = v.into();
        }

        if let Ok(mut v) = var("KAS_CONFIG_MODE") {
            v.make_ascii_uppercase();
            options.config_mode = match v.as_str() {
                "READ" => ConfigMode::Read,
                "READWRITE" => ConfigMode::ReadWrite,
                "WRITEDEFAULT" => ConfigMode::WriteDefault,
                other => {
                    log::error!("from_env: bad var KAS_CONFIG_MODE={other}");
                    log::error!("from_env: supported config modes: READ, READWRITE, WRITEDEFAULT");
                    options.config_mode
                }
            };
        }

        options
    }

    /// Load/save and apply theme config on start
    ///
    /// Requires feature "serde" to load/save config.
    pub fn init_theme_config<DS: DrawSharedImpl, T: Theme<DS>>(
        &self,
        theme: &mut T,
    ) -> Result<(), Error> {
        match self.config_mode {
            #[cfg(feature = "serde")]
            ConfigMode::Read | ConfigMode::ReadWrite if self.theme_config_path.is_file() => {
                let config: T::Config = Format::guess_and_read_path(&self.theme_config_path)?;
                config.apply_startup();
                // Ignore Action: UI isn't built yet
                let _ = theme.apply_config(&config);
            }
            #[cfg(feature = "serde")]
            ConfigMode::WriteDefault if !self.theme_config_path.as_os_str().is_empty() => {
                let config = theme.config();
                config.apply_startup();
                if let Err(error) =
                    Format::guess_and_write_path(&self.theme_config_path, config.as_ref())
                {
                    warn_about_error("failed to write default config: ", &error);
                }
            }
            _ => theme.config().apply_startup(),
        }

        Ok(())
    }

    /// Load/save KAS config on start
    ///
    /// Requires feature "serde" to load/save config.
    pub fn read_config(&self) -> Result<kas::event::Config, Error> {
        #[cfg(feature = "serde")]
        if !self.config_path.as_os_str().is_empty() {
            return match self.config_mode {
                #[cfg(feature = "serde")]
                ConfigMode::Read | ConfigMode::ReadWrite => {
                    Ok(Format::guess_and_read_path(&self.config_path)?)
                }
                #[cfg(feature = "serde")]
                ConfigMode::WriteDefault => {
                    let config: kas::event::Config = Default::default();
                    if let Err(error) = Format::guess_and_write_path(&self.config_path, &config) {
                        warn_about_error("failed to write default config: ", &error);
                    }
                    Ok(config)
                }
            };
        }

        Ok(Default::default())
    }

    /// Save all config (on exit or after changes)
    ///
    /// Requires feature "serde" to save config.
    pub fn write_config<DS: DrawSharedImpl, T: Theme<DS>>(
        &self,
        _config: &kas::event::Config,
        _theme: &T,
    ) -> Result<(), Error> {
        #[cfg(feature = "serde")]
        if self.config_mode == ConfigMode::ReadWrite {
            if !self.config_path.as_os_str().is_empty() && _config.is_dirty() {
                Format::guess_and_write_path(&self.config_path, &_config)?;
            }
            let theme_config = _theme.config();
            if !self.theme_config_path.as_os_str().is_empty() && theme_config.is_dirty() {
                Format::guess_and_write_path(&self.theme_config_path, theme_config.as_ref())?;
            }
        }

        Ok(())
    }
}
