// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::cell::RefCell;

use super::{Theme, Window};
use crate::config::{Config, WindowConfig};
use crate::draw::{color, DrawIface, DrawSharedImpl};
use crate::event::EventState;
use crate::theme::ThemeDraw;

/// As [`Theme`], but without associated types
///
/// This trait is implemented automatically for all implementations of
/// [`Theme`]. It is intended only for use where a less parameterised
/// trait is required.
pub trait ThemeDst<DS: DrawSharedImpl> {
    /// Theme initialisation
    ///
    /// See also [`Theme::init`].
    fn init(&mut self, config: &RefCell<Config>);

    /// Construct per-window storage
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&mut self, config: &WindowConfig) -> Box<dyn Window>;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    fn update_window(&mut self, window: &mut dyn Window, config: &WindowConfig);

    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut dyn Window,
    ) -> Box<dyn ThemeDraw + 'a>;

    /// Background colour
    ///
    /// See also [`Theme::clear_color`].
    fn clear_color(&self) -> color::Rgba;
}

impl<DS: DrawSharedImpl, T: Theme<DS>> ThemeDst<DS> for T {
    fn init(&mut self, config: &RefCell<Config>) {
        self.init(config);
    }

    fn new_window(&mut self, config: &WindowConfig) -> Box<dyn Window> {
        let window = <T as Theme<DS>>::new_window(self, config);
        Box::new(window)
    }

    fn update_window(&mut self, window: &mut dyn Window, config: &WindowConfig) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, config);
    }

    fn draw<'b>(
        &'b self,
        draw: DrawIface<'b, DS>,
        ev: &'b mut EventState,
        window: &'b mut dyn Window,
    ) -> Box<dyn ThemeDraw + 'b> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        Box::new(<T as Theme<DS>>::draw(self, draw, ev, window))
    }

    fn clear_color(&self) -> color::Rgba {
        self.clear_color()
    }
}
