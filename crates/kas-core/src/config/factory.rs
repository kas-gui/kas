// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration options

#[cfg(feature = "serde")] use super::Format;
use super::{Config, Error};
#[cfg(feature = "serde")]
use crate::util::warn_about_error_with_path;
use std::cell::RefCell;
#[cfg(feature = "serde")] use std::path::PathBuf;
use std::rc::Rc;

/// A factory able to source and (optionally) save [`Config`]
pub trait ConfigFactory {
    /// Construct a [`Config`] object
    ///
    /// Returning an [`Error`] here will prevent startup of the UI. As such,
    /// it may be preferable to return [`Config::default()`] than to fail.
    fn read_config(&mut self) -> Result<Rc<RefCell<Config>>, Error>;

    /// Return optional config-writing fn
    fn writer(self) -> Option<Box<dyn FnMut(&Config)>>;
}

/// Always use default [`Config`]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DefaultFactory;

impl ConfigFactory for DefaultFactory {
    fn read_config(&mut self) -> Result<Rc<RefCell<Config>>, Error> {
        Ok(Rc::new(RefCell::new(Config::default())))
    }

    fn writer(self) -> Option<Box<dyn FnMut(&Config)>> {
        None
    }
}

/// Config mode
///
/// See [`ReadWriteFactory::from_env()`] documentation.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConfigMode {
    /// Automatically determine mode based on the path
    Auto,
    /// Read-only mode
    Read,
    /// Read-write mode
    ///
    /// This mode reads config on start and writes changes on exit.
    ReadWrite,
    /// Use default config and write out
    ///
    /// This mode only writes initial (default) config and any writes changes on exit.
    WriteDefault,
}

/// Read and write config from disk
#[cfg(feature = "serde")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReadWriteFactory {
    path: PathBuf,
    mode: ConfigMode,
    fail_on_error: bool,
}

#[cfg(feature = "serde")]
impl ReadWriteFactory {
    /// Construct with specified `path` and `mode`
    pub fn new(path: PathBuf, mode: ConfigMode) -> Self {
        let fail_on_error = false;
        ReadWriteFactory {
            path,
            mode,
            fail_on_error,
        }
    }

    /// Fail immediately in case of read error
    ///
    /// By default, [`Config::default`] will be returned on read error.
    pub fn fail_on_error(mut self) -> Self {
        self.fail_on_error = true;
        self
    }

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
    /// The `KAS_CONFIG_MODE` variable determines the read/write mode:
    ///
    /// -   `Read`: read-only
    /// -   `ReadWrite`: read on start-up, write on exit
    /// -   `WriteDefault`: generate platform-default configuration and write
    ///     it to the config path(s) specified, overwriting any existing config
    /// -   If not specified the mode is automatically determined depending on
    ///     what `path` resolves to.
    ///
    /// If `KAS_CONFIG_FAIL_ON_ERROR` is true, config read errors are fatal.
    /// Otherwise default configuration will be used on read error.
    pub fn from_env() -> Self {
        use std::env::var;

        let mut path = PathBuf::new();
        let mut mode = ConfigMode::Auto;
        let mut fail_on_error = false;

        if let Ok(v) = var("KAS_CONFIG") {
            path = v.into();
        }

        if let Ok(mut v) = var("KAS_CONFIG_MODE") {
            v.make_ascii_uppercase();
            mode = match v.as_str() {
                "READ" => ConfigMode::Read,
                "READWRITE" => ConfigMode::ReadWrite,
                "WRITEDEFAULT" => ConfigMode::WriteDefault,
                other => {
                    log::error!("from_env: bad var KAS_CONFIG_MODE={other}");
                    log::error!("from_env: supported config modes: READ, READWRITE, WRITEDEFAULT");
                    mode
                }
            };
        }

        if let Ok(v) = var("KAS_CONFIG_FAIL_ON_ERROR") {
            fail_on_error = match v.parse() {
                Ok(b) => b,
                _ => {
                    log::error!("from_env: bad var KAS_CONFIG_FAIL_ON_ERROR={v}");
                    true
                }
            };
        }

        ReadWriteFactory {
            path,
            mode,
            fail_on_error,
        }
    }
}

#[cfg(feature = "serde")]
impl ConfigFactory for ReadWriteFactory {
    fn read_config(&mut self) -> Result<Rc<RefCell<Config>>, Error> {
        let config = match self.mode {
            _ if self.path.as_os_str().is_empty() => Config::default(),
            ConfigMode::Auto | ConfigMode::Read | ConfigMode::ReadWrite => {
                match Format::guess_and_read_path(&self.path) {
                    Ok(config) => {
                        self.mode = match std::fs::metadata(&self.path) {
                            Ok(meta) if meta.is_file() && !meta.permissions().readonly() => {
                                ConfigMode::ReadWrite
                            }
                            _ => ConfigMode::Read,
                        };

                        config
                    }
                    Err(error) => {
                        if matches!(&error, Error::IoError(e) if e.kind() == std::io::ErrorKind::NotFound)
                        {
                            self.mode = ConfigMode::WriteDefault;
                        } else {
                            warn_about_error_with_path("failed to read config", &error, &self.path);
                            if self.fail_on_error {
                                return Err(error);
                            }
                        }
                        Config::default()
                    }
                }
            }
            ConfigMode::WriteDefault => Default::default(),
        };

        if self.mode == ConfigMode::WriteDefault {
            if let Err(error) = Format::guess_and_write_path(&self.path, &config) {
                self.mode = ConfigMode::Read;
                warn_about_error_with_path("failed to write default config: ", &error, &self.path);
            }
        }

        Ok(Rc::new(RefCell::new(config)))
    }

    fn writer(self) -> Option<Box<dyn FnMut(&Config)>> {
        if self.path.as_os_str().is_empty()
            || matches!(self.mode, ConfigMode::Read | ConfigMode::ReadWrite)
        {
            return None;
        }

        let path = self.path;
        Some(Box::new(move |config| {
            if let Err(error) = Format::guess_and_write_path(&path, config) {
                warn_about_error_with_path("failed to write config: ", &error, &path);
            }
        }))
    }
}

/// A selected [`ConfigFactory`] implementation
///
/// This is a newtype over an implementation of [`ConfigFactory`], dependent on
/// feature flags. Currently, this uses:
///
/// -   `cfg(feature = "serde")`: `ReadWriteFactory::from_env()`
/// -   Otherwise: [`DefaultFactory::default()`]
#[cfg(not(feature = "serde"))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AutoFactory(DefaultFactory);

/// A selected [`ConfigFactory`] implementation
///
/// This is a newtype over an implementation of [`ConfigFactory`], dependent on
/// feature flags. Currently, this uses:
///
/// -   `cfg(feature = "serde")`: [`ReadWriteFactory::from_env()`]
/// -   Otherwise: [`DefaultFactory::default()`]
#[cfg(feature = "serde")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AutoFactory(ReadWriteFactory);

#[cfg(feature = "serde")]
impl Default for AutoFactory {
    fn default() -> Self {
        AutoFactory(ReadWriteFactory::from_env())
    }
}

impl ConfigFactory for AutoFactory {
    #[inline]
    fn read_config(&mut self) -> Result<Rc<RefCell<Config>>, Error> {
        self.0.read_config()
    }

    #[inline]
    fn writer(self) -> Option<Box<dyn FnMut(&Config)>> {
        self.0.writer()
    }
}
