// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Top-level configuration struct

use super::{EventConfig, EventConfigMsg, EventWindowConfig};
use super::{FontConfig, FontConfigMsg, ThemeConfig, ThemeConfigMsg};
use crate::cast::Cast;
use crate::config::Shortcuts;
use crate::Action;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// A message which may be used to update [`Config`]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ConfigMsg {
    Event(EventConfigMsg),
    Font(FontConfigMsg),
    Theme(ThemeConfigMsg),
}

/// Base configuration
///
/// This is serializable (using `feature = "serde"`) with the following fields:
///
/// > `event`: [`EventConfig`] \
/// > `font`: [`FontConfig`] \
/// > `shortcuts`: [`Shortcuts`] \
/// > `theme`: [`ThemeConfig`]
///
/// For descriptions of configuration effects, see [`WindowConfig`] methods.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    pub event: EventConfig,

    pub font: FontConfig,

    #[cfg_attr(feature = "serde", serde(default = "Shortcuts::platform_defaults"))]
    pub shortcuts: Shortcuts,

    pub theme: ThemeConfig,

    #[cfg_attr(feature = "serde", serde(default = "defaults::frame_dur_nanos"))]
    frame_dur_nanos: u32,

    #[cfg_attr(feature = "serde", serde(skip))]
    is_dirty: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            event: EventConfig::default(),
            font: Default::default(),
            shortcuts: Shortcuts::platform_defaults(),
            theme: Default::default(),
            frame_dur_nanos: defaults::frame_dur_nanos(),
            is_dirty: false,
        }
    }
}

impl Config {
    /// Call on startup
    pub(crate) fn init(&mut self) {
        self.font.init();
    }

    /// Has the config ever been updated?
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
}

/// Configuration, adapted for the application and window scale
#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub(super) config: Rc<RefCell<Config>>,
    pub(super) scale_factor: f32,
    pub(super) scroll_flick_sub: f32,
    pub(super) scroll_dist: f32,
    pub(super) pan_dist_thresh: f32,
    /// Whether navigation focus is enabled for this application window
    pub(crate) nav_focus: bool,
    pub(super) frame_dur: Duration,
}

impl WindowConfig {
    /// Construct
    ///
    /// It is required to call [`Self::update`] before usage.
    pub(crate) fn new(config: Rc<RefCell<Config>>) -> Self {
        WindowConfig {
            config,
            scale_factor: f32::NAN,
            scroll_flick_sub: f32::NAN,
            scroll_dist: f32::NAN,
            pan_dist_thresh: f32::NAN,
            nav_focus: true,
            frame_dur: Default::default(),
        }
    }

    /// Update window-specific/cached values
    pub(crate) fn update(&mut self, scale_factor: f32) {
        let base = self.config.borrow();
        self.scale_factor = scale_factor;
        self.scroll_flick_sub = base.event.scroll_flick_sub * scale_factor;
        let dpem = base.font.size() * scale_factor;
        self.scroll_dist = base.event.scroll_dist_em * dpem;
        self.pan_dist_thresh = base.event.pan_dist_thresh * scale_factor;
        self.frame_dur = Duration::from_nanos(base.frame_dur_nanos.cast());
    }

    /// Access base (unscaled) [`Config`]
    pub fn base(&self) -> Ref<Config> {
        self.config.borrow()
    }

    /// Update the base config
    ///
    /// Since it is not known which parts of the configuration are updated, all
    /// configuration-update [`Action`]s must be performed.
    ///
    /// NOTE: adjusting font settings from a running app is not currently
    /// supported, excepting font size.
    ///
    /// NOTE: it is assumed that widget state is not affected by config except
    /// (a) state affected by a widget update (e.g. the `EventConfig` widget)
    /// and (b) widget size may be affected by font size.
    pub fn update_base<F: FnOnce(&mut Config)>(&self, f: F) -> Action {
        if let Ok(mut c) = self.config.try_borrow_mut() {
            c.is_dirty = true;

            let font_size = c.font.size();
            f(&mut c);

            let mut action = Action::EVENT_CONFIG | Action::THEME_UPDATE;
            if c.font.size() != font_size {
                action |= Action::RESIZE;
            }
            action
        } else {
            Action::empty()
        }
    }

    /// Access event config
    pub fn event(&self) -> EventWindowConfig {
        EventWindowConfig(self)
    }

    /// Update event configuration
    pub fn update_event<F: FnOnce(&mut EventConfig) -> Action>(&self, f: F) -> Action {
        if let Ok(mut c) = self.config.try_borrow_mut() {
            c.is_dirty = true;
            f(&mut c.event)
        } else {
            Action::empty()
        }
    }

    /// Access font config
    pub fn font(&self) -> Ref<FontConfig> {
        Ref::map(self.config.borrow(), |c| &c.font)
    }

    /// Set standard font size
    ///
    /// Units: logical (unscaled) pixels per Em.
    ///
    /// To convert to Points, multiply by three quarters.
    ///
    /// NOTE: this is currently the only supported run-time update to font configuration.
    pub fn set_font_size(&self, pt_size: f32) -> Action {
        if let Ok(mut c) = self.config.try_borrow_mut() {
            c.is_dirty = true;
            c.font.set_size(pt_size)
        } else {
            Action::empty()
        }
    }

    /// Access shortcut config
    pub fn shortcuts(&self) -> Ref<Shortcuts> {
        Ref::map(self.config.borrow(), |c| &c.shortcuts)
    }

    /// Access theme config
    pub fn theme(&self) -> Ref<ThemeConfig> {
        Ref::map(self.config.borrow(), |c| &c.theme)
    }

    /// Update theme configuration
    pub fn update_theme<F: FnOnce(&mut ThemeConfig) -> Action>(&self, f: F) -> Action {
        if let Ok(mut c) = self.config.try_borrow_mut() {
            c.is_dirty = true;

            f(&mut c.theme)
        } else {
            Action::empty()
        }
    }

    /// Adjust shortcuts
    pub fn update_shortcuts<F: FnOnce(&mut Shortcuts)>(&self, f: F) -> Action {
        if let Ok(mut c) = self.config.try_borrow_mut() {
            c.is_dirty = true;

            f(&mut c.shortcuts);
            Action::UPDATE
        } else {
            Action::empty()
        }
    }

    /// Scale factor
    pub fn scale_factor(&self) -> f32 {
        debug_assert!(self.scale_factor.is_finite());
        self.scale_factor
    }

    /// Update event configuration via a [`ConfigMsg`]
    pub fn change_config(&self, msg: ConfigMsg) -> Action {
        match msg {
            ConfigMsg::Event(msg) => self.update_event(|ev| ev.change_config(msg)),
            ConfigMsg::Font(FontConfigMsg::Size(size)) => self.set_font_size(size),
            ConfigMsg::Theme(msg) => self.update_theme(|theme| theme.change_config(msg)),
        }
    }

    /// Minimum frame time
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[inline]
    pub fn frame_dur(&self) -> Duration {
        self.frame_dur
    }
}

mod defaults {
    pub fn frame_dur_nanos() -> u32 {
        12_500_000 // 1e9 / 80
    }
}
