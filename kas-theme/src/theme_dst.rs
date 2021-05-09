// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::any::Any;
use std::borrow::Cow;
use std::ops::DerefMut;

use super::{StackDst, Theme, Window};
use kas::draw::{Colour, DrawHandle, DrawShared, SizeHandle, ThemeApi};
use kas::TkAction;

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
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub trait ThemeDst<D: DrawShared>: ThemeApi {
    /// Get current config
    fn config(&self) -> MaybeBoxed<dyn Any>;

    /// Apply/set the passed config
    fn apply_config(&mut self, config: &dyn Any) -> TkAction;

    /// Theme initialisation
    ///
    /// See also [`Theme::init`].
    fn init(&mut self, draw: &mut D);

    /// Construct per-window storage
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&self, draw: &mut D::Draw, dpi_factor: f32) -> StackDst<dyn WindowDst<D>>;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    fn update_window(&self, window: &mut dyn WindowDst<D>, dpi_factor: f32);

    /// Construct a [`DrawHandle`] object
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::draw_handle`].
    ///
    /// This function is **unsafe** because the returned object requires a
    /// lifetime bound not exceeding that of all three pointers passed in.
    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        shared: &mut D,
        draw: &mut D::Draw,
        window: &mut dyn WindowDst<D>,
    ) -> StackDst<dyn DrawHandle>;

    /// Construct a [`DrawHandle`] object
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::draw_handle`].
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        shared: &'a mut D,
        draw: &'a mut D::Draw,
        window: &'a mut dyn WindowDst<D>,
    ) -> StackDst<dyn DrawHandle + 'a>;

    /// Background colour
    ///
    /// See also [`Theme::clear_color`].
    fn clear_color(&self) -> Colour;
}

#[cfg(not(feature = "gat"))]
impl<'a, D: DrawShared, T: Theme<D>> ThemeDst<D> for T
where
    <T as Theme<D>>::DrawHandle: 'static,
    <<T as Theme<D>>::Window as Window<D>>::SizeHandle: 'static,
{
    fn config(&self) -> MaybeBoxed<dyn Any> {
        match self.config() {
            Cow::Borrowed(config) => MaybeBoxed::Borrowed(config),
            Cow::Owned(config) => MaybeBoxed::Boxed(Box::new(config.to_owned())),
        }
    }

    fn apply_config(&mut self, config: &dyn Any) -> TkAction {
        self.apply_config(config.downcast_ref().unwrap())
    }

    fn init(&mut self, draw: &mut D) {
        self.init(draw);
    }

    fn new_window(&self, draw: &mut D::Draw, dpi_factor: f32) -> StackDst<dyn WindowDst<D>> {
        let window = <T as Theme<D>>::new_window(self, draw, dpi_factor);
        #[cfg(feature = "unsize")]
        {
            StackDst::new_or_boxed(window)
        }
        #[cfg(not(feature = "unsize"))]
        {
            match StackDst::new_stable(window, |w| w as &dyn WindowDst<D>) {
                Ok(s) => s,
                Err(window) => StackDst::new_stable(Box::new(window), |w| w as &dyn WindowDst<D>)
                    .ok()
                    .expect("boxed window too big for StackDst!"),
            }
        }
    }

    fn update_window(&self, window: &mut dyn WindowDst<D>, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
    }

    unsafe fn draw_handle(
        &self,
        shared: &mut D,
        draw: &mut D::Draw,
        window: &mut dyn WindowDst<D>,
    ) -> StackDst<dyn DrawHandle> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<D>>::draw_handle(self, shared, draw, window);
        #[cfg(feature = "unsize")]
        {
            StackDst::new_or_boxed(h)
        }
        #[cfg(not(feature = "unsize"))]
        {
            StackDst::new_stable(h, |h| h as &dyn DrawHandle)
                .ok()
                .expect("handle too big for StackDst!")
        }
    }

    fn clear_color(&self) -> Colour {
        self.clear_color()
    }
}

#[cfg(feature = "gat")]
impl<'a, D: DrawShared, T: Theme<D>> ThemeDst<D> for T {
    fn config(&self) -> MaybeBoxed<dyn Any> {
        match self.config() {
            Cow::Borrowed(config) => MaybeBoxed::Borrowed(config),
            Cow::Owned(config) => MaybeBoxed::Boxed(Box::new(config.to_owned())),
        }
    }

    fn apply_config(&mut self, config: &dyn Any) -> TkAction {
        self.apply_config(config.downcast_ref().unwrap())
    }

    fn init(&mut self, draw: &mut D) {
        self.init(draw);
    }

    fn new_window(&self, draw: &mut D::Draw, dpi_factor: f32) -> StackDst<dyn WindowDst<D>> {
        let window = <T as Theme<D>>::new_window(self, draw, dpi_factor);
        StackDst::new_or_boxed(window)
    }

    fn update_window(&self, window: &mut dyn WindowDst<D>, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
    }

    fn draw_handle<'b>(
        &'b self,
        shared: &'b mut D,
        draw: &'b mut D::Draw,
        window: &'b mut dyn WindowDst<D>,
    ) -> StackDst<dyn DrawHandle + 'b> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<D>>::draw_handle(self, shared, draw, window);
        StackDst::new_or_boxed(h)
    }

    fn clear_color(&self) -> Colour {
        self.clear_color()
    }
}

/// As [`Window`], but without associated types
///
/// This trait is implemented automatically for all implementations of
/// [`Window`]. It is intended only for use where a less parameterised
/// trait is required.
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub trait WindowDst<D: DrawShared> {
    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    ///
    /// This function is **unsafe** because the returned object requires a
    /// lifetime bound not exceeding that of all three pointers passed in.
    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self, draw: &mut D) -> StackDst<dyn SizeHandle>;

    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self, draw: &'a mut D) -> StackDst<dyn SizeHandle + 'a>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[cfg(not(feature = "gat"))]
impl<D: DrawShared, W: Window<D>> WindowDst<D> for W
where
    <W as Window<D>>::SizeHandle: 'static,
{
    unsafe fn size_handle<'a>(&'a mut self, draw: &'a mut D) -> StackDst<dyn SizeHandle> {
        let h = <W as Window<D>>::size_handle(self, draw);
        #[cfg(feature = "unsize")]
        {
            StackDst::new_or_boxed(h)
        }
        #[cfg(not(feature = "unsize"))]
        {
            StackDst::new_stable(h, |h| h as &dyn SizeHandle)
                .ok()
                .expect("handle too big for StackDst!")
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.as_any_mut()
    }
}

#[cfg(feature = "gat")]
impl<D: DrawShared, W: Window<D>> WindowDst<D> for W {
    fn size_handle<'a>(&'a mut self, draw: &'a mut D) -> StackDst<dyn SizeHandle + 'a> {
        let h = <W as Window<D>>::size_handle(self, draw);
        StackDst::new_or_boxed(h)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.as_any_mut()
    }
}

impl<D: DrawShared> Window<D> for StackDst<dyn WindowDst<D>> {
    #[cfg(not(feature = "gat"))]
    type SizeHandle = StackDst<dyn SizeHandle>;
    #[cfg(feature = "gat")]
    type SizeHandle<'a> = StackDst<dyn SizeHandle + 'a>;

    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self, draw: &mut D) -> Self::SizeHandle {
        self.deref_mut().size_handle(draw)
    }

    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self, draw: &'a mut D) -> Self::SizeHandle<'a> {
        self.deref_mut().size_handle(draw)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }
}
