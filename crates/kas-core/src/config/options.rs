// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration options

use super::{Config, Error};
#[cfg(feature = "serde")] use super::{Format, ThemeConfig};
use crate::draw::DrawSharedImpl;
use crate::theme::Theme;
#[cfg(feature = "serde")] use crate::util::warn_about_error;
use std::env::var;
use std::path::PathBuf;

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
                let config: ThemeConfig = Format::guess_and_read_path(&self.theme_config_path)?;
                // Ignore Action: UI isn't built yet
                let _ = theme.apply_config(&config);
            }
            #[cfg(feature = "serde")]
            ConfigMode::WriteDefault if !self.theme_config_path.as_os_str().is_empty() => {
                let config = theme.config();
                if let Err(error) =
                    Format::guess_and_write_path(&self.theme_config_path, config.as_ref())
                {
                    warn_about_error("failed to write default config: ", &error);
                }
            }
            _ => {
                let _ = theme;
            }
        }

        Ok(())
    }

    /// Load/save KAS config on start
    ///
    /// Requires feature "serde" to load/save config.
    pub fn read_config(&self) -> Result<Config, Error> {
        #[cfg(feature = "serde")]
        if !self.config_path.as_os_str().is_empty() {
            return match self.config_mode {
                #[cfg(feature = "serde")]
                ConfigMode::Read | ConfigMode::ReadWrite => {
                    Ok(Format::guess_and_read_path(&self.config_path)?)
                }
                #[cfg(feature = "serde")]
                ConfigMode::WriteDefault => {
                    let config: Config = Default::default();
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
        _config: &Config,
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
