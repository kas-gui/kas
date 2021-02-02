// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme traits

use std::any::Any;
use std::ops::{Deref, DerefMut};

use kas::draw::{Colour, DrawHandle, DrawShared, SizeHandle};
use kas::geom::Rect;
use kas::ThemeApi;

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `Draw` type.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme<D: DrawShared>: ThemeApi {
    /// The associated [`Window`] implementation.
    type Window: Window + 'static;

    /// The associated [`DrawHandle`] implementation.
    #[cfg(not(feature = "gat"))]
    type DrawHandle: DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a>: DrawHandle;

    /// Theme initialisation
    ///
    /// The toolkit must call this method before [`Theme::new_window`]
    /// to allow initialisation specific to the `Draw` device.
    ///
    /// At a minimum, a theme must load a font to [`kas::text::fonts`].
    /// The first font loaded (by any theme) becomes the default font.
    fn init(&mut self, draw: &mut D);

    /// Construct per-window storage
    ///
    /// On "standard" monitors, the `dpi_factor` is 1. High-DPI screens may
    /// have a factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = i32::conv_nearest(MARGIN * factor);
    /// ```
    ///
    /// A reference to the draw backend is provided allowing configuration.
    fn new_window(&self, draw: &mut D::Draw, dpi_factor: f32) -> Self::Window;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// This is called when the DPI factor changes or theme dimensions change.
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32);

    /// Prepare to draw and construct a [`DrawHandle`] object
    ///
    /// This is called once per window per frame and should do any necessary
    /// preparation such as loading fonts and textures which are loaded on
    /// demand.
    ///
    /// Drawing via this [`DrawHandle`] is restricted to the specified `rect`.
    ///
    /// The `window` is guaranteed to be one created by a call to
    /// [`Theme::new_window`] on `self`, and the `draw` reference is guaranteed
    /// to be identical to the one passed to [`Theme::new_window`].
    ///
    /// This method is marked *unsafe* since a lifetime restriction is required
    /// on the return value which can only be expressed with the unstable
    /// feature Generic Associated Types (rust#44265).
    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: &mut D::Draw,
        window: &mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle;
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a>;

    /// Background colour
    fn clear_color(&self) -> Colour;
}

/// Per-window storage for the theme
///
/// Constructed via [`Theme::new_window`].
///
/// The main reason for this separation is to allow proper handling of
/// multi-window applications across screens with differing DPIs.
pub trait Window {
    /// The associated [`SizeHandle`] implementation.
    #[cfg(not(feature = "gat"))]
    type SizeHandle: SizeHandle;
    #[cfg(feature = "gat")]
    type SizeHandle<'a>: SizeHandle;

    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self) -> Self::SizeHandle;
    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self) -> Self::SizeHandle<'a>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Theme<D>, D: DrawShared> Theme<D> for Box<T> {
    type Window = <T as Theme<D>>::Window;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = <T as Theme<D>>::DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = <T as Theme<D>>::DrawHandle<'a>;

    fn init(&mut self, draw: &mut D) {
        self.deref_mut().init(draw);
    }

    fn new_window(&self, draw: &mut D::Draw, dpi_factor: f32) -> Self::Window {
        self.deref().new_window(draw, dpi_factor)
    }
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        self.deref().update_window(window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: &mut D::Draw,
        window: &mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        self.deref().draw_handle(draw, window, rect)
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a> {
        self.deref().draw_handle(draw, window, rect)
    }

    fn clear_color(&self) -> Colour {
        self.deref().clear_color()
    }
}

impl<W: Window> Window for Box<W> {
    #[cfg(not(feature = "gat"))]
    type SizeHandle = <W as Window>::SizeHandle;
    #[cfg(feature = "gat")]
    type SizeHandle<'a> = <W as Window>::SizeHandle<'a>;

    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle(&mut self) -> Self::SizeHandle {
        self.deref_mut().size_handle()
    }
    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self) -> Self::SizeHandle<'a> {
        self.deref_mut().size_handle()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.deref_mut().as_any_mut()
    }
}
