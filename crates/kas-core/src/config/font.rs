// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Font configuration

use crate::text::fonts::{self, AddMode, FontSelector};
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
///
/// Note that only changes to [`Self::size`] are currently supported at run-time.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontConfig {
    /// Standard font size, in units of pixels-per-Em
    #[cfg_attr(feature = "serde", serde(default = "defaults::size"))]
    pub size: f32,

    /// Font aliases, used when searching for a font family matching the key.
    ///
    /// Example:
    /// ```yaml
    /// aliases:
    ///   sans-serif:
    ///     mode: Prepend
    ///     list:
    ///     - noto sans
    /// ```
    ///
    /// Fonts are named by *family*. Several standard families exist, e.g.
    /// "serif", "sans-serif", "monospace"; these resolve to a list
    /// of aliases (e.g. "Noto Sans", "DejaVu Sans", "Arial"), each of which may
    /// have further aliases.
    ///
    /// In the above example, "noto sans" is inserted at the top of the alias
    /// list for "sans-serif".
    ///
    /// Supported modes: `Prepend`, `Append`, `Replace`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub aliases: BTreeMap<String, FontAliases>,

    /// Standard fonts
    #[cfg_attr(feature = "serde", serde(default))]
    pub fonts: BTreeMap<TextClass, FontSelector<'static>>,

    /// Text glyph rastering settings
    #[cfg_attr(feature = "serde", serde(default))]
    pub raster: RasterConfig,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            size: defaults::size(),
            aliases: Default::default(),
            fonts: defaults::fonts(),
            raster: Default::default(),
        }
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub mode: u8,
    /// Scale multiplier for fixed-precision
    ///
    /// This should be an integer `n >= 1`, e.g. `n = 4` provides four sub-pixel
    /// steps of precision. It is also required that `n * h < (1 << 24)` where
    /// `h` is the text height in pixels.
    #[cfg_attr(feature = "serde", serde(default = "defaults::scale_steps"))]
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
    #[cfg_attr(feature = "serde", serde(default = "defaults::subpixel_threshold"))]
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
    #[cfg_attr(feature = "serde", serde(default = "defaults::subpixel_steps"))]
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
impl FontConfig {
    /// Standard font size
    ///
    /// Units: logical (unscaled) pixels per Em.
    ///
    /// To convert to Points, multiply by three quarters.
    #[inline]
    pub fn size(&self) -> f32 {
        self.size
    }

    /// Get an iterator over font mappings
    #[inline]
    pub fn iter_fonts(&self) -> impl Iterator<Item = (&TextClass, &FontSelector<'static>)> {
        self.fonts.iter()
    }
}

/// Setters
impl FontConfig {
    /// Set standard font size
    ///
    /// Units: logical (unscaled) pixels per Em.
    ///
    /// To convert to Points, multiply by three quarters.
    pub fn set_size(&mut self, pt_size: f32) {
        self.size = pt_size;
    }
}

/// Other functions
impl FontConfig {
    /// Apply config effects which only happen on startup
    pub(super) fn init(&self) {
        if !self.aliases.is_empty() {
            fonts::library().update_db(|db| {
                for (family, aliases) in self.aliases.iter() {
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
    pub fn raster(&self) -> &RasterConfig {
        &self.raster
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontAliases {
    #[cfg_attr(feature = "serde", serde(default = "defaults::add_mode"))]
    mode: AddMode,
    list: Vec<String>,
}

mod defaults {
    use super::*;

    #[cfg(feature = "serde")]
    pub fn add_mode() -> AddMode {
        AddMode::Prepend
    }

    pub fn size() -> f32 {
        16.0
    }

    pub fn fonts() -> BTreeMap<TextClass, FontSelector<'static>> {
        let mut selector = FontSelector::new();
        selector.set_families(vec!["serif".into()]);
        let list = [
            (TextClass::Edit(false), selector.clone()),
            (TextClass::Edit(true), selector),
        ];
        list.iter().cloned().collect()
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
