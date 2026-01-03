// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper around mutliple themes, supporting run-time switching

use super::{ColorsLinear, Theme, ThemeDst, Window};
use crate::config::{Config, WindowConfig};
use crate::draw::{DrawIface, DrawSharedImpl, color};
use crate::event::EventState;
use crate::theme::ThemeDraw;
use std::cell::RefCell;
use std::collections::HashMap;

type DynTheme<DS> = Box<dyn ThemeDst<DS>>;

/// Run-time switching support for pre-loaded themes
pub struct MultiTheme<DS> {
    names: HashMap<String, usize>,
    themes: Vec<DynTheme<DS>>,
    active: usize,
}

impl<DS> MultiTheme<DS> {
    /// Construct an empty `MultiTheme`
    ///
    /// **At least one theme must be added before the UI starts.**.
    pub fn new() -> MultiTheme<DS> {
        MultiTheme {
            names: HashMap::new(),
            themes: vec![],
            active: 0,
        }
    }

    /// Add a theme
    ///
    /// Returns the index of the new theme.
    pub fn add<S: ToString, T>(&mut self, name: S, theme: T) -> usize
    where
        DS: DrawSharedImpl,
        T: ThemeDst<DS> + 'static,
    {
        let index = self.themes.len();
        self.names.insert(name.to_string(), index);
        self.themes.push(Box::new(theme));
        index
    }

    /// Set the active theme
    ///
    /// If this is not called, then the first theme added will be active.
    ///
    /// An invalid index will cause the UI to panic on start.
    pub fn set_active(&mut self, index: usize) {
        self.active = index;
    }
}

impl<DS: DrawSharedImpl> Theme<DS> for MultiTheme<DS> {
    type Window = Box<dyn Window>;
    type Draw<'a> = Box<dyn ThemeDraw + 'a>;

    fn init(&mut self, config: &RefCell<Config>) {
        if self.active >= self.themes.len() {
            panic!(
                "MultiTheme: invalid index {} in list of {} themes added",
                self.active,
                self.themes.len()
            );
        }

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
        if let Some(index) = self.names.get(theme).cloned()
            && index != self.active
        {
            self.active = index;
        }

        self.themes[self.active].new_window(config)
    }

    fn update_window(&mut self, window: &mut Self::Window, config: &WindowConfig) -> bool {
        self.themes[self.active].update_window(window, config)
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
