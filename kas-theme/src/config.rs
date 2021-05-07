// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme configuration

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use kas::TkAction;

/// Event handling configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Standard font size
    ///
    /// Units: points per Em. Pixel size depends on the screen's scale factor.
    #[cfg_attr(feature = "serde", serde(default = "defaults::font_size"))]
    pub font_size: f32,

    /// Active colour scheme (name)
    ///
    /// An empty string will resolve the default colour scheme.
    #[cfg_attr(feature = "serde", serde(default))]
    pub color_scheme: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            font_size: defaults::font_size(),
            color_scheme: Default::default(),
        }
    }
}

impl Config {
    pub fn action_from_diff(&self, other: &Config) -> TkAction {
        if self.font_size != other.font_size {
            TkAction::RESIZE | TkAction::THEME_UPDATE
        } else if self != other {
            TkAction::REDRAW
        } else {
            TkAction::empty()
        }
    }
}

mod defaults {
    pub fn font_size() -> f32 {
        12.0
    }
}
