// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme configuration

use crate::{ThemeColours, ThemeConfig};
use kas::TkAction;
use std::collections::BTreeMap;

/// Event handling configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(serde::Serialize, serde::Deserialize))]
pub struct Config {
    #[cfg_attr(feature = "config", serde(skip))]
    dirty: bool,

    #[cfg_attr(feature = "config", serde(default = "defaults::font_size"))]
    font_size: f32,

    #[cfg_attr(feature = "config", serde(default))]
    active_scheme: String,

    /// TODO: possibly we should not save default schemes and merge when
    /// loading (perhaps via a `PartialConfig` type).
    #[cfg_attr(feature = "config", serde(default = "defaults::color_schemes",))]
    color_schemes: BTreeMap<String, ThemeColours>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dirty: false,
            font_size: defaults::font_size(),
            active_scheme: Default::default(),
            color_schemes: defaults::color_schemes(),
        }
    }
}

/// Getters
impl Config {
    /// Standard font size
    ///
    /// Units: points per Em. Pixel size depends on the screen's scale factor.
    #[inline]
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Active colour scheme (name)
    ///
    /// An empty string will resolve the default colour scheme.
    #[inline]
    pub fn active_scheme(&self) -> &str {
        &self.active_scheme
    }

    /// Iterate over all colour schemes
    #[inline]
    pub fn color_schemes_iter(&self) -> impl Iterator<Item = (&str, &ThemeColours)> {
        self.color_schemes.iter().map(|(s, t)| (s.as_str(), t))
    }

    /// Get a colour scheme by name
    #[inline]
    pub fn get_color_scheme(&self, name: &str) -> Option<ThemeColours> {
        self.color_schemes.get(name).cloned()
    }

    /// Get the active colour scheme
    ///
    /// Even this one isn't guaranteed to exist.
    #[inline]
    pub fn get_active_scheme(&self) -> Option<ThemeColours> {
        self.color_schemes.get(&self.active_scheme).cloned()
    }
}

/// Setters
impl Config {
    /// Set font size
    pub fn set_font_size(&mut self, pt_size: f32) {
        self.dirty = true;
        self.font_size = pt_size;
    }

    /// Set colour scheme
    pub fn set_active_scheme(&mut self, scheme: impl ToString) {
        self.dirty = true;
        self.active_scheme = scheme.to_string();
    }
}

/// Other functions
impl Config {
    /// Currently this is just "set". Later, maybe some type of merge.
    pub fn apply_config(&mut self, other: &Config) -> TkAction {
        let action = if self.font_size != other.font_size {
            TkAction::RESIZE | TkAction::THEME_UPDATE
        } else if self != other {
            TkAction::REDRAW
        } else {
            TkAction::empty()
        };

        *self = other.clone();
        action
    }
}

impl ThemeConfig for Config {
    /// Has the config ever been updated?
    #[inline]
    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

mod defaults {
    use super::*;

    pub fn font_size() -> f32 {
        12.0
    }

    pub fn color_schemes() -> BTreeMap<String, ThemeColours> {
        let mut schemes = BTreeMap::new();
        schemes.insert("".to_string(), ThemeColours::white_blue());
        schemes.insert("light".to_string(), ThemeColours::light());
        schemes.insert("dark".to_string(), ThemeColours::dark());
        schemes
    }
}
