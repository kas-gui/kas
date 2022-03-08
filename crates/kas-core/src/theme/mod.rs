// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme APIs

mod draw;
mod size;
mod style;

pub use draw::{Background, DrawHandle, DrawMgr};
pub use size::{SizeHandle, SizeMgr};
pub use style::*;

#[allow(unused)]
use crate::event::EventMgr;
use crate::geom::{Coord, Rect};
use crate::{CoreData, TkAction, WidgetId};
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

/// Widget identifier and coordinate for a drawn feature
pub struct IdCoord<'a>(pub &'a WidgetId, pub Coord);

impl<'a> From<&'a CoreData> for IdCoord<'a> {
    fn from(core: &CoreData) -> IdCoord {
        IdCoord(&core.id, core.rect.pos)
    }
}

impl<'a, W: crate::WidgetCore> From<&'a W> for IdCoord<'a> {
    fn from(w: &W) -> IdCoord {
        let core = w.core_data();
        IdCoord(&core.id, core.rect.pos)
    }
}

impl<'a, W: crate::WidgetCore> From<&'a mut W> for IdCoord<'a> {
    fn from(w: &mut W) -> IdCoord {
        let core = w.core_data();
        IdCoord(&core.id, core.rect.pos)
    }
}

/// Widget identifier and rect for a drawn feature
pub struct IdRect<'a>(pub &'a WidgetId, pub Rect);

impl<'a> From<&'a CoreData> for IdRect<'a> {
    fn from(core: &CoreData) -> IdRect {
        IdRect(&core.id, core.rect)
    }
}

impl<'a, W: crate::WidgetCore> From<&'a W> for IdRect<'a> {
    fn from(w: &W) -> IdRect {
        let core = w.core_data();
        IdRect(&core.id, core.rect)
    }
}

impl<'a, W: crate::WidgetCore> From<&'a mut W> for IdRect<'a> {
    fn from(w: &mut W) -> IdRect {
        let core = w.core_data();
        IdRect(&core.id, core.rect)
    }
}
