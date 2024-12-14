// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration formats and read/write support

#[cfg(feature = "serde")]
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use thiserror::Error;

/// Configuration read/write/format errors
#[derive(Error, Debug)]
pub enum Error {
    #[cfg(feature = "yaml")]
    #[error("config deserialisation failed")]
    De(#[from] serde::de::value::Error),

    #[cfg(feature = "yaml")]
    #[error("config serialisation to YAML failed")]
    YamlSer(#[from] serde_yaml2::ser::Errors),

    #[cfg(feature = "json")]
    #[error("config (de)serialisation to JSON failed")]
    Json(#[from] serde_json::Error),

    #[cfg(feature = "ron")]
    #[error("config serialisation to RON failed")]
    Ron(#[from] ron::Error),

    #[cfg(feature = "ron")]
    #[error("config deserialisation from RON failed")]
    RonSpanned(#[from] ron::error::SpannedError),

    #[cfg(feature = "toml")]
    #[error("config deserialisation from TOML failed")]
    TomlDe(#[from] toml::de::Error),

    #[cfg(feature = "toml")]
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
                let contents = std::fs::read_to_string(path)?;
                Ok(serde_yaml2::from_str(&contents)?)
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
                let text = serde_yaml2::to_string(value)?;
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
