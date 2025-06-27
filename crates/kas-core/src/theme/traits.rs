// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme traits

use super::{ColorsLinear, ThemeDraw, ThemeSize};
use crate::autoimpl;
use crate::config::{Config, WindowConfig};
use crate::draw::{DrawIface, DrawSharedImpl, color};
use crate::event::EventState;
use std::any::Any;
use std::cell::RefCell;

#[allow(unused)] use crate::event::EventCx;

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `DrawIface`.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Theme<DS: DrawSharedImpl> {
    /// The associated [`Window`] implementation.
    type Window: Window;

    /// The associated [`ThemeDraw`] implementation.
    type Draw<'a>: ThemeDraw
    where
        DS: 'a,
        Self: 'a;

    /// Theme initialisation
    ///
    /// The toolkit must call this method before [`Theme::new_window`]
    /// to allow initialisation specific to the `DrawIface`.
    fn init(&mut self, config: &RefCell<Config>);

    /// Construct per-window storage
    ///
    /// Updates theme from configuration and constructs a scaled per-window size
    /// cache.
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
    fn new_window(&mut self, config: &WindowConfig) -> Self::Window;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// This is called when the DPI factor changes or theme config or dimensions change.
    fn update_window(&mut self, window: &mut Self::Window, config: &WindowConfig);

    /// Prepare to draw and construct a [`ThemeDraw`] object
    ///
    /// This is called once per window per frame and should do any necessary
    /// preparation such as loading fonts and textures which are loaded on
    /// demand.
    ///
    /// Drawing via this [`ThemeDraw`] object is restricted to the specified `rect`.
    ///
    /// The `window` is guaranteed to be one created by a call to
    /// [`Theme::new_window`] on `self`.
    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut Self::Window,
    ) -> Self::Draw<'a>;

    /// Construct a draw object from parts
    ///
    /// This method allows a "derived" theme to construct a draw object for the
    /// inherited theme.
    fn draw_upcast<'a>(
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        w: &'a mut Self::Window,
        cols: &'a ColorsLinear,
    ) -> Self::Draw<'a>;

    /// The window/scene clear color
    ///
    /// This is not used when the window is transparent.
    fn clear_color(&self) -> color::Rgba;
}

/// Per-window storage for the theme
///
/// Constructed via [`Theme::new_window`].
///
/// The main reason for this separation is to allow proper handling of
/// multi-window applications across screens with differing DPIs.
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Window: 'static {
    /// Construct a [`ThemeSize`] object
    fn size(&self) -> &dyn ThemeSize;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}
