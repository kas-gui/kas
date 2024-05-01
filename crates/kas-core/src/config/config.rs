// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Top-level configuration struct

use super::event::{self, ChangeConfig};
use super::FontConfig;
use crate::cast::Cast;
use crate::config::Shortcuts;
use crate::Action;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Base configuration
///
/// This is serializable (using `feature = "serde"`) with the following fields:
///
/// > `event`: [`event::Config`] \
/// > `shortcuts`: [`Shortcuts`]
///
/// For descriptions of configuration effects, see [`WindowConfig`] methods.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    pub event: event::Config,

    pub font: FontConfig,

    #[cfg_attr(feature = "serde", serde(default = "Shortcuts::platform_defaults"))]
    pub shortcuts: Shortcuts,

    #[cfg_attr(feature = "serde", serde(default = "defaults::frame_dur_nanos"))]
    frame_dur_nanos: u32,

    #[cfg_attr(feature = "serde", serde(skip))]
    is_dirty: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            event: event::Config::default(),
            font: Default::default(),
            shortcuts: Shortcuts::platform_defaults(),
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

    pub(crate) fn change_config(&mut self, msg: event::ChangeConfig) {
        self.event.change_config(msg);
        self.is_dirty = true;
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
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
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
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn update(&mut self, scale_factor: f32) {
        let base = self.config.borrow();
        self.scale_factor = scale_factor;
        self.scroll_flick_sub = base.event.scroll_flick_sub * scale_factor;
        let dpem = base.font.size() * scale_factor;
        self.scroll_dist = base.event.scroll_dist_em * dpem;
        self.pan_dist_thresh = base.event.pan_dist_thresh * scale_factor;
        self.frame_dur = Duration::from_nanos(base.frame_dur_nanos.cast());
    }

    /// Borrow access to the [`Config`]
    pub fn borrow(&self) -> Ref<Config> {
        self.config.borrow()
    }

    /// Update event configuration
    #[inline]
    pub fn change_config(&mut self, msg: ChangeConfig) -> Action {
        match self.config.try_borrow_mut() {
            Ok(mut config) => {
                config.change_config(msg);
                Action::EVENT_CONFIG
            }
            Err(_) => {
                log::error!("WindowConfig::change_config: failed to mutably borrow config");
                Action::empty()
            }
        }
    }
}

impl WindowConfig {
    /// Access event config
    pub fn event(&self) -> event::WindowConfig {
        event::WindowConfig(self)
    }

    /// Access font config
    pub fn font(&self) -> Ref<FontConfig> {
        Ref::map(self.config.borrow(), |c| &c.font)
    }

    /// Access shortcut config
    pub fn shortcuts(&self) -> Ref<Shortcuts> {
        Ref::map(self.config.borrow(), |c| &c.shortcuts)
    }

    /// Scale factor
    pub fn scale_factor(&self) -> f32 {
        debug_assert!(self.scale_factor.is_finite());
        self.scale_factor
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
