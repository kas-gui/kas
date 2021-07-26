// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper around mutliple themes, supporting run-time switching

use std::collections::HashMap;
#[cfg(feature = "unsize")]
use std::marker::Unsize;

use crate::{Config, StackDst, Theme, ThemeDst, WindowDst};
use kas::draw::{color, Draw, DrawHandle, DrawShared, DrawableShared, ThemeApi};
use kas::TkAction;

#[cfg(feature = "unsize")]
type DynTheme<DS> = StackDst<dyn ThemeDst<DS>>;
#[cfg(not(feature = "unsize"))]
type DynTheme<DS> = Box<dyn ThemeDst<DS>>;

/// Wrapper around mutliple themes, supporting run-time switching
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub struct MultiTheme<DS> {
    names: HashMap<String, usize>,
    themes: Vec<DynTheme<DS>>,
    active: usize,
}

/// Builder for [`MultiTheme`]
///
/// Construct via [`MultiTheme::builder`].
pub struct MultiThemeBuilder<DS> {
    names: HashMap<String, usize>,
    themes: Vec<DynTheme<DS>>,
}

impl<DS> MultiTheme<DS> {
    /// Construct with builder pattern
    pub fn builder() -> MultiThemeBuilder<DS> {
        MultiThemeBuilder {
            names: HashMap::new(),
            themes: vec![],
        }
    }
}

impl<DS> MultiThemeBuilder<DS> {
    /// Add a theme
    ///
    /// Note: the constraints of this method vary depending on the `unsize`
    /// feature.
    #[cfg(feature = "unsize")]
    pub fn add<S: ToString, U>(mut self, name: S, theme: U) -> Self
    where
        U: Unsize<dyn ThemeDst<DS>>,
        Box<U>: Unsize<dyn ThemeDst<DS>>,
    {
        let index = self.themes.len();
        self.names.insert(name.to_string(), index);
        self.themes.push(StackDst::new_or_boxed(theme));
        self
    }

    /// Add a theme
    ///
    /// Note: the constraints of this method vary depending on the `unsize`
    /// feature.
    #[cfg(not(feature = "unsize"))]
    pub fn add<S: ToString, T>(mut self, name: S, theme: T) -> Self
    where
        DS: DrawableShared,
        T: ThemeDst<DS> + 'static,
    {
        let index = self.themes.len();
        self.names.insert(name.to_string(), index);
        self.themes.push(Box::new(theme));
        self
    }

    /// Build
    ///
    /// Returns `None` if no themes were added.
    pub fn try_build(self) -> Option<MultiTheme<DS>> {
        if self.themes.is_empty() {
            return None;
        }
        Some(MultiTheme {
            names: self.names,
            themes: self.themes,
            active: 0,
        })
    }

    /// Build
    ///
    /// Panics if no themes were added.
    pub fn build(self) -> MultiTheme<DS> {
        self.try_build()
            .unwrap_or_else(|| panic!("MultiThemeBuilder: no themes added"))
    }
}

impl<DS: DrawableShared> Theme<DS> for MultiTheme<DS> {
    type Config = Config;
    type Window = StackDst<dyn WindowDst>;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = StackDst<dyn DrawHandle>;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = StackDst<dyn DrawHandle + 'a>;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        let boxed_config = self.themes[self.active].config();
        // TODO: write each sub-theme's config instead of this stupid cast!
        let config: Config = boxed_config
            .as_ref()
            .downcast_ref::<Config>()
            .unwrap()
            .clone();
        std::borrow::Cow::Owned(config)
    }

    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        let mut action = TkAction::empty();
        for theme in &mut self.themes {
            action |= theme.apply_config(config);
        }
        action
    }

    fn init(&mut self, shared: &mut DrawShared<DS>) {
        for theme in &mut self.themes {
            theme.init(shared);
        }
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        self.themes[self.active].new_window(dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        self.themes[self.active].update_window(window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        shared: &mut DrawShared<DS>,
        draw: Draw<DS::Draw>,
        window: &mut Self::Window,
    ) -> StackDst<dyn DrawHandle> {
        unsafe fn extend_lifetime_mut<'b, T: ?Sized>(r: &'b mut T) -> &'static mut T {
            std::mem::transmute::<&'b mut T, &'static mut T>(r)
        }
        self.themes[self.active].draw_handle(
            extend_lifetime_mut(shared),
            Draw::new(extend_lifetime_mut(draw.draw), draw.pass()),
            extend_lifetime_mut(window),
        )
    }

    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        shared: &'a mut DrawShared<DS>,
        draw: Draw<'a, DS::Draw>,
        window: &'a mut Self::Window,
    ) -> StackDst<dyn DrawHandle + 'a> {
        self.themes[self.active].draw_handle(shared, draw, window)
    }

    fn clear_color(&self) -> color::Rgba {
        self.themes[self.active].clear_color()
    }
}

impl<DS> ThemeApi for MultiTheme<DS> {
    fn set_font_size(&mut self, size: f32) -> TkAction {
        // Slightly inefficient, but sufficient: update both
        // (Otherwise we would have to call set_scheme in set_theme too.)
        let mut action = TkAction::empty();
        for theme in &mut self.themes {
            action = action.max(theme.set_font_size(size));
        }
        action
    }

    fn set_scheme(&mut self, scheme: &str) -> TkAction {
        // Slightly inefficient, but sufficient: update all
        // (Otherwise we would have to call set_scheme in set_theme too.)
        let mut action = TkAction::empty();
        for theme in &mut self.themes {
            action = action.max(theme.set_scheme(scheme));
        }
        action
    }

    fn list_schemes(&self) -> Vec<&str> {
        // We list only schemes of the active theme. Probably all themes should
        // have the same schemes anyway.
        self.themes[self.active].list_schemes()
    }

    fn set_theme(&mut self, theme: &str) -> TkAction {
        if let Some(index) = self.names.get(theme).cloned() {
            if index != self.active {
                self.active = index;
                return TkAction::RESIZE | TkAction::THEME_UPDATE;
            }
        }
        TkAction::empty()
    }
}
