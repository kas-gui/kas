// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper around mutliple themes, supporting run-time switching

use super::{ColorsLinear, Theme, ThemeDst, Window};
use crate::config::{Config, WindowConfig};
use crate::draw::{color, DrawIface, DrawSharedImpl};
use crate::event::EventState;
use crate::theme::ThemeDraw;
use std::cell::RefCell;
use std::collections::HashMap;

type DynTheme<DS> = Box<dyn ThemeDst<DS>>;

/// Wrapper around multiple themes, supporting run-time switching
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
    #[must_use]
    pub fn add<S: ToString, T>(mut self, name: S, theme: T) -> Self
    where
        DS: DrawSharedImpl,
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

impl<DS: DrawSharedImpl> Theme<DS> for MultiTheme<DS> {
    type Window = Box<dyn Window>;
    type Draw<'a> = Box<dyn ThemeDraw + 'a>;

    fn init(&mut self, config: &RefCell<Config>) {
        if config.borrow().theme.active_theme.is_empty() {
            for (name, index) in &self.names {
                if *index == self.active {
                    let _ = config.borrow_mut().theme.set_active_theme(name.to_string());
                    break;
                }
            }
        }

        for theme in &mut self.themes {
            theme.init(config);
        }
    }

    fn new_window(&mut self, config: &WindowConfig) -> Self::Window {
        // We may switch themes here
        let theme = &config.theme().active_theme;
        if let Some(index) = self.names.get(theme).cloned() {
            if index != self.active {
                self.active = index;
            }
        }

        self.themes[self.active].new_window(config)
    }

    fn update_window(&mut self, window: &mut Self::Window, config: &WindowConfig) {
        self.themes[self.active].update_window(window, config);
    }

    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut Self::Window,
    ) -> Box<dyn ThemeDraw + 'a> {
        self.themes[self.active].draw(draw, ev, window)
    }

    fn draw_upcast<'a>(
        _draw: DrawIface<'a, DS>,
        _ev: &'a mut EventState,
        _w: &'a mut Self::Window,
        _cols: &'a ColorsLinear,
    ) -> Self::Draw<'a> {
        unimplemented!()
    }

    fn clear_color(&self) -> color::Rgba {
        self.themes[self.active].clear_color()
    }
}
