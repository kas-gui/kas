// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::any::Any;
use std::borrow::Cow;

use super::{Theme, Window};
use crate::draw::{color, DrawIface, DrawSharedImpl, SharedState};
use crate::event::EventState;
use crate::theme::{ThemeControl, ThemeDraw};
use crate::TkAction;

/// An optionally-owning (boxed) reference
///
/// This is related but not identical to [`Cow`].
pub enum MaybeBoxed<'a, B: 'a + ?Sized> {
    Borrowed(&'a B),
    Boxed(Box<B>),
}

impl<T: ?Sized> AsRef<T> for MaybeBoxed<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            MaybeBoxed::Borrowed(r) => r,
            MaybeBoxed::Boxed(b) => b.as_ref(),
        }
    }
}

/// As [`Theme`], but without associated types
///
/// This trait is implemented automatically for all implementations of
/// [`Theme`]. It is intended only for use where a less parameterised
/// trait is required.
pub trait ThemeDst<DS: DrawSharedImpl>: ThemeControl {
    /// Get current configuration
    fn config(&self) -> MaybeBoxed<dyn Any>;

    /// Apply/set the passed config
    fn apply_config(&mut self, config: &dyn Any) -> TkAction;

    /// Theme initialisation
    ///
    /// See also [`Theme::init`].
    fn init(&mut self, shared: &mut SharedState<DS>);

    /// Construct per-window storage
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&self, dpi_factor: f32) -> Box<dyn Window>;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    fn update_window(&self, window: &mut dyn Window, dpi_factor: f32);

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
    fn config(&self) -> MaybeBoxed<dyn Any> {
        match self.config() {
            Cow::Borrowed(config) => MaybeBoxed::Borrowed(config),
            Cow::Owned(config) => MaybeBoxed::Boxed(Box::new(config)),
        }
    }

    fn apply_config(&mut self, config: &dyn Any) -> TkAction {
        self.apply_config(config.downcast_ref().unwrap())
    }

    fn init(&mut self, shared: &mut SharedState<DS>) {
        self.init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> Box<dyn Window> {
        let window = <T as Theme<DS>>::new_window(self, dpi_factor);
        Box::new(window)
    }

    fn update_window(&self, window: &mut dyn Window, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
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
