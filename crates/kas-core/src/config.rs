// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration read/write utilities
#![cfg_attr(doc_cfg, doc(cfg(feature = "config")))]

use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use thiserror::Error;

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
    #[error("config (de)serialisation to RON failed")]
    Ron(#[from] dep_ron::Error),

    #[error("error reading / writing config file")]
    IoError(#[from] std::io::Error),

    #[error("format not supported: {0}")]
    UnsupportedFormat(Format),
}

/// Configuration serialisation formats
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Error)]
pub enum Format {
    /// Not specified: guess from the path
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

impl Default for Format {
    fn default() -> Self {
        Format::None
    }
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
    pub fn read_path<T: DeserializeOwned>(self, path: &Path) -> Result<T, Error> {
        log::info!("read: path={}, format={:?}", path.display(), self);
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
                Ok(dep_ron::de::from_reader(r)?)
            }
            _ => {
                let _ = path; // squelch unused warning
                Err(Error::UnsupportedFormat(self))
            }
        }
    }

    /// Write to a path
    pub fn write_path<T: Serialize>(self, path: &Path, value: &T) -> Result<(), Error> {
        log::info!("write: path={}, format={:?}", path.display(), self);
        match self {
            #[cfg(feature = "json")]
            Format::Json => {
                let w = std::io::BufWriter::new(std::fs::File::create(path)?);
                serde_json::to_writer_pretty(w, value)?;
                Ok(())
            }
            #[cfg(feature = "yaml")]
            Format::Yaml => {
                let w = std::io::BufWriter::new(std::fs::File::create(path)?);
                serde_yaml::to_writer(w, value)?;
                Ok(())
            }
            #[cfg(feature = "ron")]
            Format::Ron => {
                let w = std::io::BufWriter::new(std::fs::File::create(path)?);
                let pretty = dep_ron::ser::PrettyConfig::default();
                dep_ron::ser::to_writer_pretty(w, value, pretty)?;
                Ok(())
            }
            // NOTE: Toml is not supported since the `toml` crate does not support enums as map keys
            _ => {
                let _ = (path, value); // squelch unused warnings
                Err(Error::UnsupportedFormat(self))
            }
        }
    }

    /// Guess format and load from a path
    #[inline]
    pub fn guess_and_read_path<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
        let format = Self::guess_from_path(path);
        format.read_path(path)
    }

    /// Guess format and write to a path
    #[inline]
    pub fn guess_and_write_path<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
        let format = Self::guess_from_path(path);
        format.write_path(path, value)
    }
}
