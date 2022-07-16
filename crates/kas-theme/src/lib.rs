// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS theme support
//!
//! This crate provides the [`Theme`] trait, [`MultiTheme`] adapter, color
//! schemes, some supporting items, and the themes [`FlatTheme`] and
//! [`ShadedTheme`].
//!
//! Custom themes may be built over this crate, optionally including custom draw
//! routines (e.g. [`DrawShaded`]), provided that the shell implements support.
//! Alternatively this crate may be skipped altogether, especially for a
//! minimal shell with a custom fixed theme.

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(feature = "gat", feature(generic_associated_types))]

mod anim;
mod colors;
mod config;
mod draw_shaded;
mod flat_theme;
mod multi;
mod shaded_theme;
mod simple_theme;
mod theme_dst;
mod traits;

pub mod dim;

pub use colors::{Colors, ColorsLinear, ColorsSrgb, InputState};
pub use config::{Config, RasterConfig};
pub use draw_shaded::{DrawShaded, DrawShadedImpl};
pub use flat_theme::FlatTheme;
pub use multi::{MultiTheme, MultiThemeBuilder};
pub use shaded_theme::ShadedTheme;
pub use simple_theme::SimpleTheme;
pub use theme_dst::{MaybeBoxed, ThemeDst};
pub use traits::{Theme, ThemeConfig, Window};
