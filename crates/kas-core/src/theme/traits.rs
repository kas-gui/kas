// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Theme traits

use super::{ColorsLinear, ColorsSrgb, RasterConfig, ThemeDraw, ThemeSize};
use crate::draw::{color, DrawIface, DrawSharedImpl, SharedState};
use crate::event::EventState;
use crate::{autoimpl, Action};
use std::any::Any;

#[allow(unused)] use crate::event::EventMgr;

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`Action`] to enable correct action when a theme
/// is updated via [`EventMgr::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
#[crate::autoimpl(for<T: trait + ?Sized> &mut T, Box<T>)]
pub trait ThemeControl {
    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    fn set_font_size(&mut self, pt_size: f32) -> Action;

    /// Get the name of the active color scheme
    fn active_scheme(&self) -> &str;

    /// List available color schemes
    fn list_schemes(&self) -> Vec<&str>;

    /// Get colors of a named scheme
    fn get_scheme(&self, name: &str) -> Option<&ColorsSrgb>;

    /// Access the in-use color scheme
    fn get_colors(&self) -> &ColorsLinear;

    /// Set colors directly
    ///
    /// This may be used to provide a custom color scheme. The `name` is
    /// compulsary (and returned by [`Self::get_active_scheme`]).
    /// The `name` is also used when saving config, though the custom colors are
    /// not currently saved in this config.
    fn set_colors(&mut self, name: String, scheme: ColorsLinear) -> Action;

    /// Change the color scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    fn set_scheme(&mut self, name: &str) -> Action {
        if name != self.active_scheme() {
            if let Some(scheme) = self.get_scheme(name) {
                return self.set_colors(name.to_string(), scheme.into());
            }
        }
        Action::empty()
    }

    /// Switch the theme
    ///
    /// Most themes do not react to this method; [`super::MultiTheme`] uses
    /// it to switch themes.
    fn set_theme(&mut self, _theme: &str) -> Action {
        Action::empty()
    }
}

/// Requirements on theme config (without `config` feature)
#[cfg(not(feature = "serde"))]
pub trait ThemeConfig: Clone + std::fmt::Debug + 'static {
    /// Apply startup effects
    fn apply_startup(&self);

    /// Get raster config
    fn raster(&self) -> &RasterConfig;
}

/// Requirements on theme config (with `config` feature)
#[cfg(feature = "serde")]
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
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
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
    fn apply_config(&mut self, config: &Self::Config) -> Action;

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
