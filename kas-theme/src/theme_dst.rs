// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::any::Any;
use std::borrow::Cow;
use std::ops::DerefMut;

use super::{StackDst, Theme, Window};
use kas::draw::{color, DrawHandle, DrawShared, DrawableShared, SizeHandle, ThemeApi};
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
pub trait ThemeDst<DS: DrawableShared>: ThemeApi {
    /// Get current config
    fn config(&self) -> MaybeBoxed<dyn Any>;

    /// Apply/set the passed config
    fn apply_config(&mut self, config: &dyn Any) -> TkAction;

    /// Theme initialisation
    ///
    /// See also [`Theme::init`].
    fn init(&mut self, shared: &mut DrawShared<DS>);

    /// Construct per-window storage
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&self, dpi_factor: f32) -> StackDst<dyn WindowDst<DS>>;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    fn update_window(&self, window: &mut dyn WindowDst<DS>, dpi_factor: f32);

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
        shared: &'static mut DrawShared<DS>,
        draw: &'static mut DS::Draw,
        window: &'static mut dyn WindowDst<DS>,
    ) -> StackDst<dyn DrawHandle>;

    /// Construct a [`DrawHandle`] object
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::draw_handle`].
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        shared: &'a mut DrawShared<DS>,
        draw: &'a mut DS::Draw,
        window: &'a mut dyn WindowDst<DS>,
    ) -> StackDst<dyn DrawHandle + 'a>;

    /// Background colour
    ///
    /// See also [`Theme::clear_color`].
    fn clear_color(&self) -> color::Rgba;
}

#[cfg(not(feature = "gat"))]
impl<'a, DS: DrawableShared, T: Theme<DS>> ThemeDst<DS> for T
where
    <T as Theme<DS>>::DrawHandle: 'static,
    <<T as Theme<DS>>::Window as Window<DS>>::SizeHandle: 'static,
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

    fn init(&mut self, shared: &mut DrawShared<DS>) {
        self.init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> StackDst<dyn WindowDst<DS>> {
        let window = <T as Theme<DS>>::new_window(self, dpi_factor);
        #[cfg(feature = "unsize")]
        {
            StackDst::new_or_boxed(window)
        }
        #[cfg(not(feature = "unsize"))]
        {
            match StackDst::new_stable(window, |w| w as &dyn WindowDst<DS>) {
                Ok(s) => s,
                Err(window) => StackDst::new_stable(Box::new(window), |w| w as &dyn WindowDst<DS>)
                    .ok()
                    .expect("boxed window too big for StackDst!"),
            }
        }
    }

    fn update_window(&self, window: &mut dyn WindowDst<DS>, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
    }

    unsafe fn draw_handle(
        &self,
        shared: &'static mut DrawShared<DS>,
        draw: &'static mut DS::Draw,
        window: &'static mut dyn WindowDst<DS>,
    ) -> StackDst<dyn DrawHandle> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<DS>>::draw_handle(self, shared, draw, window);
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

    fn clear_color(&self) -> color::Rgba {
        self.clear_color()
    }
}

#[cfg(feature = "gat")]
impl<'a, DS: DrawableShared, T: Theme<DS>> ThemeDst<DS> for T {
    fn config(&self) -> MaybeBoxed<dyn Any> {
        match self.config() {
            Cow::Borrowed(config) => MaybeBoxed::Borrowed(config),
            Cow::Owned(config) => MaybeBoxed::Boxed(Box::new(config.to_owned())),
        }
    }

    fn apply_config(&mut self, config: &dyn Any) -> TkAction {
        self.apply_config(config.downcast_ref().unwrap())
    }

    fn init(&mut self, shared: &mut DrawShared<DS>) {
        self.init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> StackDst<dyn WindowDst<DS>> {
        let window = <T as Theme<DS>>::new_window(self, dpi_factor);
        StackDst::new_or_boxed(window)
    }

    fn update_window(&self, window: &mut dyn WindowDst<DS>, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
    }

    fn draw_handle<'b>(
        &'b self,
        shared: &'b mut DrawShared<DS>,
        draw: &'b mut DS::Draw,
        window: &'b mut dyn WindowDst<DS>,
    ) -> StackDst<dyn DrawHandle + 'b> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<DS>>::draw_handle(self, shared, draw, window);
        StackDst::new_or_boxed(h)
    }

    fn clear_color(&self) -> color::Rgba {
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
pub trait WindowDst<DS: DrawableShared> {
    /// Construct a [`SizeHandle`] object
    ///
    /// The `shared` reference is guaranteed to be identical to the one used to
    /// construct this object.
    ///
    /// This function is **unsafe** because the returned object requires a
    /// lifetime bound not exceeding that of all three pointers passed in.
    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self, shared: &mut DrawShared<DS>) -> StackDst<dyn SizeHandle>;

    /// Construct a [`SizeHandle`] object
    ///
    /// The `shared` reference is guaranteed to be identical to the one used to
    /// construct this object.
    #[cfg(feature = "gat")]
    fn size_handle<'a>(
        &'a mut self,
        shared: &'a mut DrawShared<DS>,
    ) -> StackDst<dyn SizeHandle + 'a>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[cfg(not(feature = "gat"))]
impl<DS: DrawableShared, W: Window<DS>> WindowDst<DS> for W
where
    <W as Window<DS>>::SizeHandle: 'static,
{
    unsafe fn size_handle<'a>(
        &'a mut self,
        shared: &'a mut DrawShared<DS>,
    ) -> StackDst<dyn SizeHandle> {
        let h = <W as Window<DS>>::size_handle(self, shared);
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
impl<DS: DrawableShared, W: Window<DS>> WindowDst<DS> for W {
    fn size_handle<'a>(
        &'a mut self,
        shared: &'a mut DrawShared<DS>,
    ) -> StackDst<dyn SizeHandle + 'a> {
        let h = <W as Window<DS>>::size_handle(self, shared);
        StackDst::new_or_boxed(h)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.as_any_mut()
    }
}

impl<DS: DrawableShared> Window<DS> for StackDst<dyn WindowDst<DS>> {
    #[cfg(not(feature = "gat"))]
    type SizeHandle = StackDst<dyn SizeHandle>;
    #[cfg(feature = "gat")]
    type SizeHandle<'a> = StackDst<dyn SizeHandle + 'a>;

    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self, shared: &mut DrawShared<DS>) -> Self::SizeHandle {
        self.deref_mut().size_handle(shared)
    }

    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self, shared: &'a mut DrawShared<DS>) -> Self::SizeHandle<'a> {
        self.deref_mut().size_handle(shared)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }
}
