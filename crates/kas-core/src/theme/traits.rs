// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme traits

use super::{RasterConfig, ThemeControl, ThemeDraw, ThemeSize};
use crate::draw::{color, DrawIface, DrawSharedImpl, SharedState};
use crate::event::EventState;
use crate::{autoimpl, TkAction};
use std::any::Any;
use std::ops::{Deref, DerefMut};

/// Requirements on theme config (without `config` feature)
#[cfg(not(feature = "config"))]
pub trait ThemeConfig: Clone + std::fmt::Debug + 'static {
    /// Apply startup effects
    fn apply_startup(&self);

    /// Get raster config
    fn raster(&self) -> &RasterConfig;
}

/// Requirements on theme config (with `config` feature)
#[cfg(feature = "config")]
pub trait ThemeConfig:
    Clone + std::fmt::Debug + 'static + for<'a> serde::Deserialize<'a> + serde::Serialize
{
    /// Has the config ever been updated?
    fn is_dirty(&self) -> bool;

    /// Apply startup effects
    fn apply_startup(&self);

    /// Get raster config
    fn raster(&self) -> &RasterConfig;
}

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `DrawIface`.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme<DS: DrawSharedImpl>: ThemeControl {
    /// The associated config type
    type Config: ThemeConfig;

    /// The associated [`Window`] implementation.
    type Window: Window;

    /// The associated [`ThemeDraw`] implementation.
    type Draw<'a>: ThemeDraw
    where
        DS: 'a,
        Self: 'a;

    /// Get current configuration
    fn config(&self) -> std::borrow::Cow<Self::Config>;

    /// Apply/set the passed config
    fn apply_config(&mut self, config: &Self::Config) -> TkAction;

    /// Theme initialisation
    ///
    /// The toolkit must call this method before [`Theme::new_window`]
    /// to allow initialisation specific to the `DrawIface`.
    ///
    /// At a minimum, a theme must load a font to [`crate::text::fonts`].
    /// The first font loaded (by any theme) becomes the default font.
    fn init(&mut self, shared: &mut SharedState<DS>);

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
    fn new_window(&self, dpi_factor: f32) -> Self::Window;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// This is called when the DPI factor changes or theme dimensions change.
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32);

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

    /// Background colour
    fn clear_color(&self) -> color::Rgba;
}

/// Per-window storage for the theme
///
/// Constructed via [`Theme::new_window`].
///
/// The main reason for this separation is to allow proper handling of
/// multi-window applications across screens with differing DPIs.
#[autoimpl(for<T: trait> Box<T>)]
pub trait Window: 'static {
    /// Construct a [`ThemeSize`] object
    fn size(&self) -> &dyn ThemeSize;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Theme<DS>, DS: DrawSharedImpl> Theme<DS> for Box<T> {
    type Window = <T as Theme<DS>>::Window;
    type Config = <T as Theme<DS>>::Config;

    type Draw<'a> = <T as Theme<DS>>::Draw<'a>
    where
        T: 'a;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        self.deref().config()
    }
    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        self.deref_mut().apply_config(config)
    }

    fn init(&mut self, shared: &mut SharedState<DS>) {
        self.deref_mut().init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        self.deref().new_window(dpi_factor)
    }
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        self.deref().update_window(window, dpi_factor);
    }

    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut Self::Window,
    ) -> Self::Draw<'a> {
        self.deref().draw(draw, ev, window)
    }

    fn clear_color(&self) -> color::Rgba {
        self.deref().clear_color()
    }
}
