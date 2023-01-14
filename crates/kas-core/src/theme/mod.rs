// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme interface
//!
//! Includes the [`Theme`] trait.

mod anim;
mod colors;
mod config;
mod draw;
mod multi;
mod size;
mod style;
mod theme_dst;
mod traits;

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub mod dimensions;

pub use colors::{Colors, ColorsLinear, ColorsSrgb, InputState};
pub use config::{Config, RasterConfig};
pub use draw::{Background, DrawMgr, ThemeDraw};
pub use multi::{MultiTheme, MultiThemeBuilder};
pub use size::{SizeMgr, ThemeSize};
pub use style::*;
pub use theme_dst::{MaybeBoxed, ThemeDst};
pub use traits::{Theme, ThemeConfig, ThemeControl, Window};
