// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme configuration

use crate::theme::ColorsSrgb;
use std::collections::BTreeMap;
use std::time::Duration;

/// A message which may be used to update [`ThemeConfig`]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ThemeConfigMsg {
    /// Changes the active colour scheme (only if this already exists)
    SetActiveScheme(String),
    /// Adds or updates a scheme. Does not change the active scheme.
    AddScheme(String, ColorsSrgb),
    /// Removes a scheme
    RemoveScheme(String),
    /// Set the fade duration (ms)
    FadeDurationMs(u32),
}

/// Event handling configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThemeConfig {
    /// The colour scheme to use
    #[cfg_attr(feature = "serde", serde(default))]
    pub active_scheme: String,

    /// All colour schemes
    /// TODO: possibly we should not save default schemes and merge when
    /// loading (perhaps via a `PartialConfig` type).
    #[cfg_attr(feature = "serde", serde(default = "defaults::color_schemes"))]
    pub color_schemes: BTreeMap<String, ColorsSrgb>,

    /// Text cursor blink rate: delay between switching states
    #[cfg_attr(feature = "serde", serde(default = "defaults::cursor_blink_rate_ms"))]
    pub cursor_blink_rate_ms: u32,

    /// Transition duration used in animations
    #[cfg_attr(feature = "serde", serde(default = "defaults::transition_fade_ms"))]
    pub transition_fade_ms: u32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            active_scheme: defaults::default_scheme(),
            color_schemes: defaults::color_schemes(),
            cursor_blink_rate_ms: defaults::cursor_blink_rate_ms(),
            transition_fade_ms: defaults::transition_fade_ms(),
        }
    }
}

impl ThemeConfig {
    pub(super) fn change_config(&mut self, msg: ThemeConfigMsg) {
        match msg {
            ThemeConfigMsg::SetActiveScheme(scheme) => self.set_active_scheme(scheme),
            ThemeConfigMsg::AddScheme(scheme, colors) => self.add_scheme(scheme, colors),
            ThemeConfigMsg::RemoveScheme(scheme) => self.remove_scheme(&scheme),
            ThemeConfigMsg::FadeDurationMs(dur) => self.transition_fade_ms = dur,
        }
    }
}

impl ThemeConfig {
    /// Active colour scheme (name)
    ///
    /// An empty string will resolve the default colour scheme.
    #[inline]
    pub fn active_scheme(&self) -> &str {
        &self.active_scheme
    }

    /// Set the active colour scheme (by name)
    ///
    /// Does nothing if the named scheme is not found.
    pub fn set_active_scheme(&mut self, scheme: impl ToString) {
        let scheme = scheme.to_string();
        if self.color_schemes.keys().any(|k| *k == scheme) {
            self.active_scheme = scheme.to_string();
        }
    }

    /// Iterate over all colour schemes
    #[inline]
    pub fn color_schemes(&self) -> impl Iterator<Item = (&str, &ColorsSrgb)> {
        self.color_schemes.iter().map(|(s, t)| (s.as_str(), t))
    }

    /// Get a colour scheme by name
    #[inline]
    pub fn get_color_scheme(&self, name: &str) -> Option<&ColorsSrgb> {
        self.color_schemes.get(name)
    }

    /// Get the active colour scheme
    #[inline]
    pub fn get_active_scheme(&self) -> &ColorsSrgb {
        self.color_schemes
            .get(&self.active_scheme)
            .unwrap_or(&ColorsSrgb::LIGHT)
    }

    /// Add or update a colour scheme
    pub fn add_scheme(&mut self, scheme: impl ToString, colors: ColorsSrgb) {
        self.color_schemes.insert(scheme.to_string(), colors);
    }

    /// Remove a colour scheme
    pub fn remove_scheme(&mut self, scheme: &str) {
        self.color_schemes.remove(scheme);
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

mod defaults {
    use super::*;

    #[cfg(not(feature = "dark-light"))]
    pub fn default_scheme() -> String {
        "light".to_string()
    }

    #[cfg(feature = "dark-light")]
    pub fn default_scheme() -> String {
        use dark_light::Mode;
        match dark_light::detect() {
            Mode::Dark => "dark".to_string(),
            Mode::Light | Mode::Default => "light".to_string(),
        }
    }

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
