// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme traits

use rusttype::Font;
use std::any::Any;
use std::ops::{Deref, DerefMut};

use super::{DrawHandle, SizeHandle, ThemeAction};
use kas::draw::Colour;
use kas::geom::Rect;

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`ThemeAction`] to enable correct action when a theme
/// is updated via [`Manager::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
///
/// [`Manager::adjust_theme`]: crate::event::Manager::adjust_theme
pub trait ThemeApi {
    /// Set font size. Default is 18. Units are unknown.
    fn set_font_size(&mut self, size: f32) -> ThemeAction;

    /// Change the colour scheme
    ///
    /// If no theme by this name is found, the theme is unchanged.
    // TODO: revise scheme identification and error handling?
    fn set_colours(&mut self, _scheme: &str) -> ThemeAction;

    /// Change the theme itself
    ///
    /// Themes may do nothing, or may react according to their own
    /// interpretation of this method.
    fn set_theme(&mut self, _theme: &str) -> ThemeAction {
        ThemeAction::None
    }
}

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `Draw` type.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme<Draw>: ThemeApi {
    /// The associated [`Window`] implementation.
    type Window: Window<Draw> + 'static;

    /// The associated [`DrawHandle`] implementation.
    #[cfg(not(feature = "gat"))]
    type DrawHandle: DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a>: DrawHandle;

    /// Construct per-window storage
    ///
    /// On "standard" monitors, the `dpi_factor` is 1. High-DPI screens may
    /// have a factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = (MARGIN * factor).round() as u32;
    /// ```
    ///
    /// A reference to the draw backend is provided allowing configuration.
    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> Self::Window;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// This is called when the DPI factor changes or theme dimensions change.
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32);

    /// Construct a [`DrawHandle`] object
    ///
    /// Drawing via this [`DrawHandle`] is restricted to the specified `rect`.
    ///
    /// The `window` is guaranteed to be one created by a call to
    /// [`Theme::new_window`] on `self`, and the `draw` reference is guaranteed
    /// to be identical to the one passed to [`Theme::new_window`].
    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: &mut Draw,
        window: &mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle;
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a>;

    /// Get the list of available fonts
    ///
    /// Currently, all fonts used must be specified up front by this method.
    /// (Dynamic addition of fonts may be enabled in the future.)
    ///
    /// This is considered a "getter" rather than a "constructor" method since
    /// the `Font` type is cheap to copy, and each window requires its own copy.
    /// It may also be useful to retain a `Font` handle for access to its
    /// methods.
    ///
    /// Corresponding `FontId`s may be created from the index into this list.
    /// The first font in the list will be the default font.
    ///
    /// TODO: this part of the API is dependent on `rusttype::Font`. We should
    /// build an abstraction over this, or possibly just pass the font bytes
    /// (although this makes re-use of fonts between windows difficult).
    fn get_fonts<'a>(&self) -> Vec<Font<'a>>;

    /// Light source
    ///
    /// This affects shadows on frames, etc. The light source has neutral colour
    /// and intensity such that the colour of flat surfaces is unaffected.
    ///
    /// Return value: `(a, b)` where `0 ≤ a < pi/2` is the angle to the screen
    /// normal (i.e. `a = 0` is straight at the screen) and `b` is the bearing
    /// (from UP, clockwise), both in radians.
    ///
    /// Currently this is not updated after initial set-up.
    fn light_direction(&self) -> (f32, f32);

    /// Background colour
    fn clear_colour(&self) -> Colour;
}

/// Per-window storage for the theme
///
/// Constructed via [`Theme::new_window`].
///
/// The main reason for this separation is to allow proper handling of
/// multi-window applications across screens with differing DPIs.
pub trait Window<Draw> {
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
    unsafe fn size_handle(&mut self, draw: &mut Draw) -> Self::SizeHandle;
    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self, draw: &'a mut Draw) -> Self::SizeHandle<'a>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: ThemeApi> ThemeApi for Box<T> {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        self.deref_mut().set_font_size(size)
    }
    fn set_colours(&mut self, scheme: &str) -> ThemeAction {
        self.deref_mut().set_colours(scheme)
    }
    fn set_theme(&mut self, theme: &str) -> ThemeAction {
        self.deref_mut().set_theme(theme)
    }
}

impl<T: Theme<Draw>, Draw> Theme<Draw> for Box<T> {
    type Window = <T as Theme<Draw>>::Window;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = <T as Theme<Draw>>::DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = <T as Theme<Draw>>::DrawHandle<'a>;

    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> Self::Window {
        self.deref().new_window(draw, dpi_factor)
    }
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        self.deref().update_window(window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: &mut Draw,
        window: &mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        self.deref().draw_handle(draw, window, rect)
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a> {
        self.deref().draw_handle(draw, window, rect)
    }

    fn get_fonts<'a>(&self) -> Vec<Font<'a>> {
        self.deref().get_fonts()
    }
    fn light_direction(&self) -> (f32, f32) {
        self.deref().light_direction()
    }
    fn clear_colour(&self) -> Colour {
        self.deref().clear_colour()
    }
}

impl<W: Window<Draw>, Draw> Window<Draw> for Box<W> {
    #[cfg(not(feature = "gat"))]
    type SizeHandle = <W as Window<Draw>>::SizeHandle;
    #[cfg(feature = "gat")]
    type SizeHandle<'a> = <W as Window<Draw>>::SizeHandle<'a>;

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
