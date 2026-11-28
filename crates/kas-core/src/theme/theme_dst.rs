// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::cell::RefCell;

use super::{Theme, dimensions};
use crate::ActionResize;
use crate::config::{Config, WindowConfig};
use crate::draw::{DrawIface, DrawSharedImpl, color};
use crate::event::EventState;
use crate::theme::ThemeDraw;

/// As [`Theme`], but without associated types
///
/// This trait is implemented automatically for all implementations of
/// [`Theme`]. It is intended only for use where a less parameterised
/// trait is required.
#[crate::split_impl(for<T: Theme<DS>> T)]
pub trait ThemeDst<DS: DrawSharedImpl> {
    /// Theme initialisation
    ///
    /// See also [`Theme::init`].
    fn init(&mut self, config: &RefCell<Config>) {
        self.init(config);
    }

    /// Construct per-window storage
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&mut self, config: &WindowConfig) -> dimensions::Window {
        <T as Theme<DS>>::new_window(self, config)
    }

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    ///
    /// Returns whether a resize is required based on changes to the scale factor or font size.
    fn update_window(
        &mut self,
        window: &mut dimensions::Window,
        config: &WindowConfig,
    ) -> Option<ActionResize> {
        <T as Theme<DS>>::update_window(self, window, config)
    }

    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut dimensions::Window,
    ) -> Box<dyn ThemeDraw + 'a> {
        Box::new(<T as Theme<DS>>::draw(self, draw, ev, window))
    }

    /// Background colour
    ///
    /// See also [`Theme::clear_color`].
    fn clear_color(&self) -> color::Rgba {
        self.clear_color()
    }
}
