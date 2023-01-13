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
pub use traits::{Theme, ThemeConfig, Window};

#[allow(unused)] use crate::event::EventMgr;
use crate::TkAction;
use std::ops::{Deref, DerefMut};

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`TkAction`] to enable correct action when a theme
/// is updated via [`EventMgr::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
pub trait ThemeControl {
    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    fn set_font_size(&mut self, pt_size: f32) -> TkAction;

    /// Change the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    fn set_scheme(&mut self, scheme: &str) -> TkAction;

    /// List available colour schemes
    fn list_schemes(&self) -> Vec<&str>;

    /// Switch the theme
    ///
    /// Most themes do not react to this method; `kas_theme::MultiTheme` uses
    /// it to switch themes.
    fn set_theme(&mut self, _theme: &str) -> TkAction {
        TkAction::empty()
    }
}

impl<T: ThemeControl> ThemeControl for Box<T> {
    fn set_font_size(&mut self, size: f32) -> TkAction {
        self.deref_mut().set_font_size(size)
    }
    fn set_scheme(&mut self, scheme: &str) -> TkAction {
        self.deref_mut().set_scheme(scheme)
    }
    fn list_schemes(&self) -> Vec<&str> {
        self.deref().list_schemes()
    }
    fn set_theme(&mut self, theme: &str) -> TkAction {
        self.deref_mut().set_theme(theme)
    }
}
