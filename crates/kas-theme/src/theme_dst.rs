// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::any::Any;
use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

use super::{StackDst, Theme, Window};
use kas::draw::{color, DrawIface, DrawSharedImpl, SharedState};
use kas::event::EventState;
use kas::theme::{ThemeControl, ThemeDraw, ThemeSize};
use kas::TkAction;

/// An optionally-owning (boxed) reference
///
/// This is related but not identical to [`Cow`].
#[cfg_attr(doc_cfg, doc(cfg(feature = "stack_dst")))]
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
#[cfg_attr(doc_cfg, doc(cfg(feature = "stack_dst")))]
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
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&self, dpi_factor: f32) -> StackDst<dyn Window>;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    fn update_window(&self, window: &mut dyn Window, dpi_factor: f32);

    /// Construct a [`ThemeDraw`] object
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::draw`].
    ///
    /// # Safety
    ///
    /// All references passed into the method must outlive the returned object.
    #[cfg(not(feature = "gat"))]
    unsafe fn draw(
        &self,
        draw: DrawIface<DS>,
        ev: &mut EventState,
        window: &mut dyn Window,
    ) -> StackDst<dyn ThemeDraw>;

    /// Construct a [`ThemeDraw`] object
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::draw`].
    #[cfg(feature = "gat")]
    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut dyn Window,
    ) -> StackDst<dyn ThemeDraw + 'a>;

    /// Background colour
    ///
    /// See also [`Theme::clear_color`].
    fn clear_color(&self) -> color::Rgba;
}

#[cfg(not(feature = "gat"))]
impl<DS: DrawSharedImpl, T: Theme<DS>> ThemeDst<DS> for T
where
    <T as Theme<DS>>::Draw: 'static,
{
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

    fn new_window(&self, dpi_factor: f32) -> StackDst<dyn Window> {
        let window = <T as Theme<DS>>::new_window(self, dpi_factor);
        #[cfg(feature = "unsize")]
        {
            StackDst::new_or_boxed(window)
        }
        #[cfg(not(feature = "unsize"))]
        {
            match StackDst::new_stable(window, |w| w as &dyn Window) {
                Ok(s) => s,
                Err(window) => StackDst::new_stable(Box::new(window), |w| w as &dyn Window)
                    .ok()
                    .expect("boxed window too big for StackDst!"),
            }
        }
    }

    fn update_window(&self, window: &mut dyn Window, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
    }

    unsafe fn draw(
        &self,
        draw: DrawIface<DS>,
        ev: &mut EventState,
        window: &mut dyn Window,
    ) -> StackDst<dyn ThemeDraw> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<DS>>::draw(self, draw, ev, window);
        #[cfg(feature = "unsize")]
        {
            StackDst::new_or_boxed(h)
        }
        #[cfg(not(feature = "unsize"))]
        {
            StackDst::new_stable(h, |h| h as &dyn ThemeDraw)
                .ok()
                .expect("handle too big for StackDst!")
        }
    }

    fn clear_color(&self) -> color::Rgba {
        self.clear_color()
    }
}

#[cfg(feature = "gat")]
impl<'a, DS: DrawSharedImpl, T: Theme<DS>> ThemeDst<DS> for T {
    fn config(&self) -> MaybeBoxed<dyn Any> {
        match self.config() {
            Cow::Borrowed(config) => MaybeBoxed::Borrowed(config),
            Cow::Owned(config) => MaybeBoxed::Boxed(Box::new(config.to_owned())),
        }
    }

    fn apply_config(&mut self, config: &dyn Any) -> TkAction {
        self.apply_config(config.downcast_ref().unwrap())
    }

    fn init(&mut self, shared: &mut SharedState<DS>) {
        self.init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> StackDst<dyn Window> {
        let window = <T as Theme<DS>>::new_window(self, dpi_factor);
        StackDst::new_or_boxed(window)
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
    ) -> StackDst<dyn ThemeDraw + 'b> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<DS>>::draw(self, draw, ev, window);
        StackDst::new_or_boxed(h)
    }

    fn clear_color(&self) -> color::Rgba {
        self.clear_color()
    }
}

impl Window for StackDst<dyn Window> {
    fn size(&self) -> &dyn ThemeSize {
        self.deref().size()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }
}
