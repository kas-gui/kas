// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use super::{Theme, Window};
use crate::config::{Config, ThemeConfig, WindowConfig};
use crate::draw::{color, DrawIface, DrawSharedImpl};
use crate::event::EventState;
use crate::theme::{ThemeControl, ThemeDraw};
use crate::Action;

/// As [`Theme`], but without associated types
///
/// This trait is implemented automatically for all implementations of
/// [`Theme`]. It is intended only for use where a less parameterised
/// trait is required.
pub trait ThemeDst<DS: DrawSharedImpl>: ThemeControl {
    /// Get current configuration
    fn config(&self) -> std::borrow::Cow<ThemeConfig>;

    /// Apply/set the passed config
    fn apply_config(&mut self, config: &ThemeConfig) -> Action;

    /// Theme initialisation
    ///
    /// See also [`Theme::init`].
    fn init(&mut self, config: &Config);

    /// Construct per-window storage
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&self, config: &WindowConfig) -> Box<dyn Window>;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    fn update_window(&self, window: &mut dyn Window, config: &WindowConfig);

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
    fn config(&self) -> std::borrow::Cow<ThemeConfig> {
        self.config()
    }

    fn apply_config(&mut self, config: &ThemeConfig) -> Action {
        self.apply_config(config)
    }

    fn init(&mut self, config: &Config) {
        self.init(config);
    }

    fn new_window(&self, config: &WindowConfig) -> Box<dyn Window> {
        let window = <T as Theme<DS>>::new_window(self, config);
        Box::new(window)
    }

    fn update_window(&self, window: &mut dyn Window, config: &WindowConfig) {
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
