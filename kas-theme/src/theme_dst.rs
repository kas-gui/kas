// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::any::Any;
use std::ops::DerefMut;

use rusttype::Font;

use super::{StackDst, Theme, Window};
use kas::draw::{Colour, DrawHandle, SizeHandle};
use kas::geom::Rect;
use kas::ThemeApi;

/// As [`Theme`], but without associated types
///
/// This trait is implemented automatically for all implementations of
/// [`Theme`]. It is intended only for use where a less parameterised
/// trait is required.
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub trait ThemeDst<Draw>: ThemeApi {
    /// Construct per-window storage
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::new_window`].
    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> StackDst<dyn WindowDst<Draw>>;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// See also [`Theme::update_window`].
    fn update_window(&self, window: &mut dyn WindowDst<Draw>, dpi_factor: f32);

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
        draw: &mut Draw,
        window: &mut dyn WindowDst<Draw>,
        rect: Rect,
    ) -> StackDst<dyn DrawHandle>;

    /// Construct a [`DrawHandle`] object
    ///
    /// Uses a [`StackDst`] to avoid requiring an associated type.
    ///
    /// See also [`Theme::draw_handle`].
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut Draw,
        window: &'a mut dyn WindowDst<Draw>,
        rect: Rect,
    ) -> StackDst<dyn DrawHandle + 'a>;

    /// Get the list of available fonts
    ///
    /// See also [`Theme::get_fonts`].
    fn get_fonts<'a>(&self) -> Vec<Font<'a>>;

    /// Background colour
    ///
    /// See also [`Theme::clear_colour`].
    fn clear_colour(&self) -> Colour;
}

#[cfg(not(feature = "gat"))]
impl<'a, T: Theme<Draw>, Draw> ThemeDst<Draw> for T
where
    <T as Theme<Draw>>::DrawHandle: 'static,
    <<T as Theme<Draw>>::Window as Window<Draw>>::SizeHandle: 'static,
{
    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> StackDst<dyn WindowDst<Draw>> {
        StackDst::new_or_boxed(<T as Theme<Draw>>::new_window(self, draw, dpi_factor))
    }

    fn update_window(&self, window: &mut dyn WindowDst<Draw>, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
    }

    unsafe fn draw_handle(
        &self,
        draw: &mut Draw,
        window: &mut dyn WindowDst<Draw>,
        rect: Rect,
    ) -> StackDst<dyn DrawHandle> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<Draw>>::draw_handle(self, draw, window, rect);
        StackDst::new_or_boxed(h)
    }

    fn get_fonts<'b>(&self) -> Vec<Font<'b>> {
        self.get_fonts()
    }

    fn clear_colour(&self) -> Colour {
        self.clear_colour()
    }
}

#[cfg(feature = "gat")]
impl<'a, T: Theme<Draw>, Draw> ThemeDst<Draw> for T {
    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> StackDst<dyn WindowDst<Draw>> {
        StackDst::new_or_boxed(<T as Theme<Draw>>::new_window(self, draw, dpi_factor))
    }

    fn update_window(&self, window: &mut dyn WindowDst<Draw>, dpi_factor: f32) {
        let window = window.as_any_mut().downcast_mut().unwrap();
        self.update_window(window, dpi_factor);
    }

    fn draw_handle<'b>(
        &'b self,
        draw: &'b mut Draw,
        window: &'b mut dyn WindowDst<Draw>,
        rect: Rect,
    ) -> StackDst<dyn DrawHandle + 'b> {
        let window = window.as_any_mut().downcast_mut().unwrap();
        let h = <T as Theme<Draw>>::draw_handle(self, draw, window, rect);
        StackDst::new_or_boxed(h)
    }

    fn get_fonts<'b>(&self) -> Vec<Font<'b>> {
        self.get_fonts()
    }

    fn clear_colour(&self) -> Colour {
        self.clear_colour()
    }
}

/// As [`Window`], but without associated types
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub trait WindowDst<Draw> {
    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    ///
    /// This function is **unsafe** because the returned object requires a
    /// lifetime bound not exceeding that of all three pointers passed in.
    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self, draw: &mut Draw) -> StackDst<dyn SizeHandle>;

    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self, draw: &'a mut Draw) -> StackDst<dyn SizeHandle + 'a>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[cfg(not(feature = "gat"))]
impl<W: Window<Draw>, Draw> WindowDst<Draw> for W
where
    <W as Window<Draw>>::SizeHandle: 'static,
{
    unsafe fn size_handle<'a>(&'a mut self, draw: &'a mut Draw) -> StackDst<dyn SizeHandle> {
        let h = <W as Window<Draw>>::size_handle(self, draw);
        StackDst::new_or_boxed(h)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.as_any_mut()
    }
}

#[cfg(feature = "gat")]
impl<W: Window<Draw>, Draw> WindowDst<Draw> for W {
    fn size_handle<'a>(&'a mut self, draw: &'a mut Draw) -> StackDst<dyn SizeHandle + 'a> {
        let h = <W as Window<Draw>>::size_handle(self, draw);
        StackDst::new_or_boxed(h)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.as_any_mut()
    }
}

impl<Draw> Window<Draw> for StackDst<dyn WindowDst<Draw>> {
    #[cfg(not(feature = "gat"))]
    type SizeHandle = StackDst<dyn SizeHandle>;
    #[cfg(feature = "gat")]
    type SizeHandle<'a> = StackDst<dyn SizeHandle + 'a>;

    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self, draw: &mut Draw) -> Self::SizeHandle {
        self.deref_mut().size_handle(draw)
    }

    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self, draw: &'a mut Draw) -> Self::SizeHandle<'a> {
        self.deref_mut().size_handle(draw)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }
}
