// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme configuration

use crate::theme::ColorsSrgb;
use crate::Action;
use std::collections::BTreeMap;
use std::time::Duration;

/// Event handling configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThemeConfig {
    #[cfg_attr(feature = "serde", serde(skip))]
    dirty: bool,

    /// The colour scheme to use
    #[cfg_attr(feature = "serde", serde(default))]
    active_scheme: String,

    /// All colour schemes
    /// TODO: possibly we should not save default schemes and merge when
    /// loading (perhaps via a `PartialConfig` type).
    #[cfg_attr(feature = "serde", serde(default = "defaults::color_schemes"))]
    color_schemes: BTreeMap<String, ColorsSrgb>,

    /// Text cursor blink rate: delay between switching states
    #[cfg_attr(feature = "serde", serde(default = "defaults::cursor_blink_rate_ms"))]
    cursor_blink_rate_ms: u32,

    /// Transition duration used in animations
    #[cfg_attr(feature = "serde", serde(default = "defaults::transition_fade_ms"))]
    transition_fade_ms: u32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            dirty: false,
            active_scheme: Default::default(),
            color_schemes: defaults::color_schemes(),
            cursor_blink_rate_ms: defaults::cursor_blink_rate_ms(),
            transition_fade_ms: defaults::transition_fade_ms(),
        }
    }
}

/// Getters
impl ThemeConfig {
    /// Active colour scheme (name)
    ///
    /// An empty string will resolve the default colour scheme.
    #[inline]
    pub fn active_scheme(&self) -> &str {
        &self.active_scheme
    }

    /// Iterate over all colour schemes
    #[inline]
    pub fn color_schemes_iter(&self) -> impl Iterator<Item = (&str, &ColorsSrgb)> {
        self.color_schemes.iter().map(|(s, t)| (s.as_str(), t))
    }

    /// Get a colour scheme by name
    #[inline]
    pub fn get_color_scheme(&self, name: &str) -> Option<ColorsSrgb> {
        self.color_schemes.get(name).cloned()
    }

    /// Get the active colour scheme
    ///
    /// Even this one isn't guaranteed to exist.
    #[inline]
    pub fn get_active_scheme(&self) -> Option<ColorsSrgb> {
        self.color_schemes.get(&self.active_scheme).cloned()
    }

    /// Get the cursor blink rate (delay)
    #[inline]
    pub fn cursor_blink_rate(&self) -> Duration {
        Duration::from_millis(self.cursor_blink_rate_ms as u64)
    }

    /// Get the fade duration used in transition animations
    #[inline]
    pub fn transition_fade_duration(&self) -> Duration {
        Duration::from_millis(self.transition_fade_ms as u64)
    }
}

/// Setters
impl ThemeConfig {
    /// Set colour scheme
    pub fn set_active_scheme(&mut self, scheme: impl ToString) {
        self.dirty = true;
        self.active_scheme = scheme.to_string();
    }
}

/// Other functions
impl ThemeConfig {
    /// Currently this is just "set". Later, maybe some type of merge.
    #[allow(clippy::float_cmp)]
    pub fn apply_config(&mut self, other: &ThemeConfig) -> Action {
        let action = if self != other { Action::REDRAW } else { Action::empty() };

        *self = other.clone();
        action
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }
}

mod defaults {
    use super::*;

    pub fn color_schemes() -> BTreeMap<String, ColorsSrgb> {
        let mut schemes = BTreeMap::new();
        schemes.insert("light".to_string(), ColorsSrgb::LIGHT);
        schemes.insert("dark".to_string(), ColorsSrgb::DARK);
        schemes.insert("blue".to_string(), ColorsSrgb::BLUE);
        schemes
    }

    pub fn cursor_blink_rate_ms() -> u32 {
        600
    }

    pub fn transition_fade_ms() -> u32 {
        150
    }
}
