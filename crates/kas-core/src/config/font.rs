// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Font configuration

use crate::ConfigAction;
use crate::text::fonts::FontSelector;
use crate::theme::TextClass;
use std::collections::BTreeMap;

/// A message which may be used to update [`FontConfig`]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum FontConfigMsg {
    /// Standard font size, in units of pixels-per-Em
    Size(f32),
}

/// Font configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontConfig {
    /// Standard font size, in units of pixels-per-Em
    #[cfg_attr(feature = "serde", serde(default = "defaults::size"))]
    size: f32,

    /// Standard fonts
    ///
    /// Changing this at run-tme is not currently supported.
    ///
    /// TODO: read/write support.
    #[cfg_attr(feature = "serde", serde(skip, default))]
    fonts: BTreeMap<TextClass, FontSelector>,

    /// Text glyph rastering settings
    ///
    /// Changing this at run-tme is not currently supported.
    #[cfg_attr(feature = "serde", serde(default))]
    raster: RasterConfig,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            size: defaults::size(),
            fonts: defaults::fonts(),
            raster: Default::default(),
        }
    }
}

/// Sub-pixel font rendering control
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SubpixelMode {
    /// No sub-pixel rendering
    ///
    /// This is the default because it is the simplest, always supported and never wrong.
    #[default]
    None,
    /// Horizontal RGB sub-pixels
    ///
    /// This is the most common LCD display type.
    HorizontalRGB,
}

impl SubpixelMode {
    /// Returns true if any subpixel rendering mode is enabled
    pub fn any_subpixel(self) -> bool {
        self != SubpixelMode::None
    }
}

/// Font raster settings
///
/// These are not used by the theme, but passed through to the rendering
/// backend.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RasterConfig {
    //// Raster mode/engine (backend dependent)
    ////
    /// The `mode` parameter selects rendering mode (though depending on crate
    /// features, not all options will be available). The default mode is `4`.
    /// In case the crate is built without the required `fontdue` or `swash`
    /// feature, the rasterer falls back to mode `0`.
    ///
    /// -   `mode == 0`: use `ab_glyph` for rastering
    /// -   `mode == 1`: use `ab_glyph` and align glyphs to side bearings
    /// -   `mode == 2`: deprecated
    /// -   `mode == 3`: use `swash` for rastering
    /// -   `mode == 4`: use `swash` for rastering with hinting
    #[cfg_attr(feature = "serde", serde(default = "defaults::mode"))]
    pub mode: u8,
    /// Subpixel positioning threshold
    ///
    /// Text with height `h` less than this threshold will use sub-pixel
    /// positioning (see below), which should make letter spacing more
    /// accurate for small fonts.
    ///
    /// Units: physical pixels per Em ("dpem"). Default value: 18.
    #[cfg_attr(feature = "serde", serde(default = "defaults::subpixel_threshold"))]
    pub subpixel_threshold: u8,
    /// Subpixel steps (horizontal)
    ///
    /// The number of sub-pixel positioning steps to use on the x-axis for
    /// horizontal text (when enabled; see [`Self::subpixel_threshold`]).
    ///
    /// Minimum: 1 (no sub-pixel positioning). Maximum: 16. Default: 3.
    #[cfg_attr(feature = "serde", serde(default = "defaults::subpixel_x_steps"))]
    pub subpixel_x_steps: u8,
    /// Subpixel rendering mode
    pub subpixel_mode: SubpixelMode,
}

impl Default for RasterConfig {
    fn default() -> Self {
        RasterConfig {
            mode: defaults::mode(),
            subpixel_threshold: defaults::subpixel_threshold(),
            subpixel_x_steps: defaults::subpixel_x_steps(),
            subpixel_mode: Default::default(),
        }
    }
}

/// Getters
impl FontConfig {
    /// Get font size
    ///
    /// Units: logical (unscaled) pixels per Em.
    ///
    /// To convert to Points, multiply by three quarters.
    #[inline]
    pub fn get_dpem(&self, class: TextClass) -> f32 {
        if class != TextClass::Small {
            self.size
        } else {
            self.size * 0.8
        }
    }

    /// Get a [`FontSelector`] for `class`
    #[inline]
    pub fn get_font_selector(&self, class: TextClass) -> FontSelector {
        self.fonts.get(&class).cloned().unwrap_or_default()
    }
}

/// Setters
impl FontConfig {
    /// Set standard font size
    ///
    /// Units: logical (unscaled) pixels per Em.
    ///
    /// To convert to Points, multiply by three quarters.
    pub fn set_size(&mut self, pt_size: f32) -> ConfigAction {
        if self.size != pt_size {
            self.size = pt_size;
            ConfigAction::THEME
        } else {
            ConfigAction::empty()
        }
    }
}

/// Other functions
impl FontConfig {
    /// Apply config effects which only happen on startup
    pub(super) fn init(&self) {}

    /// Get raster config
    #[inline]
    pub fn raster(&self) -> &RasterConfig {
        &self.raster
    }
}

mod defaults {
    use kas_text::fonts::FamilySelector;

    use super::*;

    pub fn size() -> f32 {
        16.0
    }

    pub fn fonts() -> BTreeMap<TextClass, FontSelector> {
        let list = [(TextClass::Editor, FamilySelector::SERIF.into())];
        list.iter().cloned().collect()
    }

    pub fn mode() -> u8 {
        4
    }
    pub fn subpixel_threshold() -> u8 {
        18
    }
    pub fn subpixel_x_steps() -> u8 {
        3
    }
}
