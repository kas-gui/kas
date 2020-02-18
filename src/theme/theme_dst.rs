// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Stack-DST versions of theme traits

use std::any::Any;
use std::ops::DerefMut;

use rusttype::Font;

use super::{DrawHandle, SizeHandle, Theme, ThemeApi, Window};
use kas::draw::Colour;
use kas::geom::Rect;

/// Fixed-size object of `Unsized` type
///
/// This is a re-export of
/// [`stack_dst::ValueA`](https://docs.rs/stack_dst/0.6.0/stack_dst/struct.ValueA.html)
/// with a custom size. The `new` and `new_or_boxed` methods provide a
/// convenient API.
pub type StackDst<T> = stack_dst::ValueA<T, [usize; 8]>;

/// As [`Theme`], but without associated types
///
/// This trait is implemented automatically for all implementations of
/// [`Theme`]. It is intended only for use where a less parameterised
/// trait is required.
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
    /// The [`StackDst`] type is unable to represent this bound.
    unsafe fn draw_handle(
        &self,
        draw: &mut Draw,
        window: &mut dyn WindowDst<Draw>,
        rect: Rect,
    ) -> StackDst<dyn DrawHandle>;

    /// Get the list of available fonts
    ///
    /// See also [`Theme::get_fonts`].
    fn get_fonts<'a>(&self) -> Vec<Font<'a>>;

    /// Light source
    ///
    /// See also [`Theme::light_direction`].
    fn light_direction(&self) -> (f32, f32);

    /// Background colour
    ///
    /// See also [`Theme::clear_colour`].
    fn clear_colour(&self) -> Colour;
}

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

    fn light_direction(&self) -> (f32, f32) {
        self.light_direction()
    }

    fn clear_colour(&self) -> Colour {
        self.clear_colour()
    }
}

/// As [`Window`], but without associated types
pub trait WindowDst<Draw> {
    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    ///
    /// Note: this function is marked **unsafe** because the returned object
    /// requires a lifetime bound not exceeding that of all three pointers
    /// passed in. This ought to be expressible using generic associated types
    /// but currently is not: https://github.com/rust-lang/rust/issues/67089
    unsafe fn size_handle(&mut self, draw: &mut Draw) -> StackDst<dyn SizeHandle>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

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

impl<Draw> Window<Draw> for StackDst<dyn WindowDst<Draw>> {
    type SizeHandle = StackDst<dyn SizeHandle>;

    unsafe fn size_handle(&mut self, draw: &mut Draw) -> Self::SizeHandle {
        self.deref_mut().size_handle(draw)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }
}
