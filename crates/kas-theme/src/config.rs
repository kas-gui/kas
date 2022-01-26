// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme configuration

use crate::{ColorsSrgb, ThemeConfig};
use kas::text::fonts::{fonts, AddMode, FontSelector};
use kas::theme::TextClass;
use kas::TkAction;
use std::collections::BTreeMap;
use std::time::Duration;

/// Event handling configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(serde::Serialize, serde::Deserialize))]
pub struct Config {
    #[cfg_attr(feature = "config", serde(skip))]
    dirty: bool,

    /// Standard font size, in units of points-per-Em
    #[cfg_attr(feature = "config", serde(default = "defaults::font_size"))]
    font_size: f32,

    /// The colour scheme to use
    #[cfg_attr(feature = "config", serde(default))]
    active_scheme: String,

    /// All colour schemes
    /// TODO: possibly we should not save default schemes and merge when
    /// loading (perhaps via a `PartialConfig` type).
    #[cfg_attr(feature = "config", serde(default = "defaults::color_schemes"))]
    color_schemes: BTreeMap<String, ColorsSrgb>,

    /// Font aliases, used when searching for a font family matching the key.
    #[cfg_attr(feature = "config", serde(default))]
    font_aliases: BTreeMap<String, FontAliases>,

    /// Standard fonts
    #[cfg_attr(feature = "config", serde(default))]
    fonts: BTreeMap<TextClass, FontSelector<'static>>,

    /// Text cursor blink rate: delay between switching states
    #[cfg_attr(feature = "config", serde(default = "defaults::cursor_blink_rate_ms"))]
    cursor_blink_rate_ms: u32,

    /// Text glyph rastering settings
    #[cfg_attr(feature = "config", serde(default))]
    raster: RasterConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dirty: false,
            font_size: defaults::font_size(),
            active_scheme: Default::default(),
            color_schemes: defaults::color_schemes(),
            font_aliases: Default::default(),
            fonts: defaults::fonts(),
            cursor_blink_rate_ms: defaults::cursor_blink_rate_ms(),
            raster: Default::default(),
        }
    }
}

/// Font raster settings
///
/// These are not used by the theme, but passed through to the rendering
/// backend.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(serde::Serialize, serde::Deserialize))]
pub struct RasterConfig {
    //// Raster mode/engine (backend dependent)
    #[cfg_attr(feature = "config", serde(default))]
    pub mode: u8,
    /// Scale multiplier for fixed-precision
    ///
    /// This should be an integer `n >= 1`, e.g. `n = 4` provides four sub-pixel
    /// steps of precision. It is also required that `n * h < (1 << 24)` where
    /// `h` is the text height in pixels.
    #[cfg_attr(feature = "config", serde(default = "defaults::scale_steps"))]
    pub scale_steps: u8,
    /// Subpixel positioning threshold
    ///
    /// Text with height `h` less than this threshold will use sub-pixel
    /// positioning, which should make letter spacing more accurate for small
    /// fonts (though exact behaviour depends on the font; it may be worse).
    /// This may make rendering worse by breaking pixel alignment.
    ///
    /// Note: this feature may not be available, depending on the backend and
    /// the mode.
    ///
    /// See also sub-pixel positioning steps.
    #[cfg_attr(feature = "config", serde(default = "defaults::subpixel_threshold"))]
    pub subpixel_threshold: u8,
    /// Subpixel steps
    ///
    /// The number of sub-pixel positioning steps to use. 1 is the minimum and
    /// equivalent to no sub-pixel positioning. 16 is the maximum.
    ///
    /// Note that since this applies to horizontal and vertical positioning, the
    /// maximum number of rastered glyphs is multiplied by the square of this
    /// value, though this maxmimum may not be reached in practice. Since this
    /// feature is usually only used for small fonts this likely acceptable.
    #[cfg_attr(feature = "config", serde(default = "defaults::subpixel_steps"))]
    pub subpixel_steps: u8,
}

impl Default for RasterConfig {
    fn default() -> Self {
        RasterConfig {
            mode: 0,
            scale_steps: defaults::scale_steps(),
            subpixel_threshold: defaults::subpixel_threshold(),
            subpixel_steps: defaults::subpixel_steps(),
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

    /// Get an iterator over font mappings
    #[inline]
    pub fn iter_fonts(&self) -> impl Iterator<Item = (&TextClass, &FontSelector<'static>)> {
        self.fonts.iter()
    }

    /// Get the cursor blink rate (delay)
    #[inline]
    pub fn cursor_blink_rate(&self) -> Duration {
        Duration::from_millis(self.cursor_blink_rate_ms as u64)
    }

    /// Get the longest animation time
    pub fn longest_animation(&self) -> Duration {
        self.cursor_blink_rate()
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
    #[allow(clippy::float_cmp)]
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
    #[cfg(feature = "config")]
    #[inline]
    fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Apply config effects which only happen on startup
    fn apply_startup(&self) {
        if !self.font_aliases.is_empty() {
            fonts().update_db(|db| {
                for (family, aliases) in self.font_aliases.iter() {
                    db.add_aliases(
                        family.to_string().into(),
                        aliases.list.iter().map(|s| s.to_string().into()),
                        aliases.mode,
                    );
                }
            });
        }
    }

    /// Get raster config
    #[inline]
    fn raster(&self) -> &RasterConfig {
        &self.raster
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(serde::Serialize, serde::Deserialize))]
pub struct FontAliases {
    #[cfg_attr(feature = "config", serde(default = "defaults::add_mode"))]
    mode: AddMode,
    list: Vec<String>,
}

mod defaults {
    use super::*;

    #[cfg(feature = "config")]
    pub fn add_mode() -> AddMode {
        AddMode::Prepend
    }

    pub fn font_size() -> f32 {
        10.0
    }

    pub fn color_schemes() -> BTreeMap<String, ColorsSrgb> {
        let mut schemes = BTreeMap::new();
        schemes.insert("light".to_string(), ColorsSrgb::light());
        schemes.insert("dark".to_string(), ColorsSrgb::dark());
        schemes.insert("blue".to_string(), ColorsSrgb::blue());
        schemes
    }

    pub fn fonts() -> BTreeMap<TextClass, FontSelector<'static>> {
        let mut selector = FontSelector::new();
        selector.set_families(vec!["serif".into()]);
        let list = [
            (TextClass::Edit, selector.clone()),
            (TextClass::EditMulti, selector),
        ];
        list.iter().cloned().collect()
    }

    pub fn cursor_blink_rate_ms() -> u32 {
        600
    }

    pub fn scale_steps() -> u8 {
        4
    }
    pub fn subpixel_threshold() -> u8 {
        0
    }
    pub fn subpixel_steps() -> u8 {
        5
    }
}
