// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

#[allow(unused)]
use crate::event::Manager;
use crate::TkAction;
use std::ops::DerefMut;

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`TkAction`] to enable correct action when a theme
/// is updated via [`Manager::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
pub trait ThemeApi {
    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    fn set_font_size(&mut self, pt_size: f32) -> TkAction;

    /// Change the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    // TODO: revise scheme identification and error handling?
    fn set_colours(&mut self, _scheme: &str) -> TkAction;

    /// Switch the theme
    ///
    /// Most themes do not react to this method; `kas_theme::MultiTheme` uses
    /// it to switch themes.
    fn set_theme(&mut self, _theme: &str) -> TkAction {
        TkAction::empty()
    }
}

impl<T: ThemeApi> ThemeApi for Box<T> {
    fn set_font_size(&mut self, size: f32) -> TkAction {
        self.deref_mut().set_font_size(size)
    }
    fn set_colours(&mut self, scheme: &str) -> TkAction {
        self.deref_mut().set_colours(scheme)
    }
    fn set_theme(&mut self, theme: &str) -> TkAction {
        self.deref_mut().set_theme(theme)
    }
}
