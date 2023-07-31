// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Themes
//!
//! Widgets expect the theme to provide an implementation of [`SizeCx`] and of
//! [`DrawCx`].
//!
//! Constructing a shell requires a [`Theme`]. Two implementations are provided
//! here: [`SimpleTheme`] and [`FlatTheme`].
//! An adapter, [`MultiTheme`], is also provided.

mod anim;
mod colors;
mod config;
mod draw;
mod flat_theme;
mod multi;
mod simple_theme;
mod size;
mod style;
mod theme_dst;
mod traits;

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub mod dimensions;

pub use colors::{Colors, ColorsLinear, ColorsSrgb, InputState};
pub use config::{Config, RasterConfig};
pub use draw::{Background, DrawCx, ThemeDraw};
pub use flat_theme::FlatTheme;
pub use multi::{MultiTheme, MultiThemeBuilder};
pub use simple_theme::SimpleTheme;
pub use size::{SizeCx, ThemeSize};
pub use style::*;
pub use theme_dst::{MaybeBoxed, ThemeDst};
pub use traits::{Theme, ThemeConfig, ThemeControl, Window};
