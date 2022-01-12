// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling configuration

use super::{shortcuts::Shortcuts, ModifiersState};
use crate::cast::Cast;
#[cfg(feature = "config")]
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Event handling configuration
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "config", derive(Serialize, Deserialize))]
pub struct Config {
    #[cfg_attr(feature = "config", serde(default = "defaults::menu_delay_ns"))]
    menu_delay_ns: u32,

    #[cfg_attr(
        feature = "config",
        serde(default = "defaults::touch_text_sel_delay_ns")
    )]
    touch_text_sel_delay_ns: u32,

    #[cfg_attr(feature = "config", serde(default = "defaults::pan_dist_thresh"))]
    pan_dist_thresh: f32,
    #[cfg_attr(feature = "config", serde(skip))]
    scaled_pan_dist_thresh: f32,

    #[cfg_attr(feature = "config", serde(default = "defaults::mouse_pan"))]
    mouse_pan: MousePan,
    #[cfg_attr(feature = "config", serde(default = "defaults::mouse_text_pan"))]
    mouse_text_pan: MousePan,

    #[cfg_attr(feature = "config", serde(default = "defaults::mouse_nav_focus"))]
    mouse_nav_focus: bool,
    #[cfg_attr(feature = "config", serde(default = "defaults::touch_nav_focus"))]
    touch_nav_focus: bool,

    #[cfg_attr(feature = "config", serde(default = "Shortcuts::platform_defaults"))]
    shortcuts: Shortcuts,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            menu_delay_ns: defaults::menu_delay_ns(),
            touch_text_sel_delay_ns: defaults::touch_text_sel_delay_ns(),
            pan_dist_thresh: defaults::pan_dist_thresh(),
            scaled_pan_dist_thresh: defaults::pan_dist_thresh(),
            mouse_pan: defaults::mouse_pan(),
            mouse_text_pan: defaults::mouse_text_pan(),
            mouse_nav_focus: defaults::mouse_nav_focus(),
            touch_nav_focus: defaults::touch_nav_focus(),
            shortcuts: Shortcuts::platform_defaults(),
        }
    }
}

/// Getters
impl Config {
    /// Set scale factor
    pub fn set_scale_factor(&mut self, factor: f32) {
        self.scaled_pan_dist_thresh = self.pan_dist_thresh * factor;
    }

    /// Delay before opening/closing menus on mouse hover
    #[inline]
    pub fn menu_delay(&self) -> Duration {
        Duration::from_nanos(self.menu_delay_ns.cast())
    }

    /// Delay before switching from panning to text-selection mode
    #[inline]
    pub fn touch_text_sel_delay(&self) -> Duration {
        Duration::from_nanos(self.touch_text_sel_delay_ns.cast())
    }

    /// Drag distance threshold before panning (scrolling) starts
    ///
    /// When the distance moved is greater than this threshold, panning should
    /// start; otherwise the system should wait for the text-selection timer.
    /// We currently recommend the L-inf distance metric (max of abs of values).
    #[inline]
    pub fn pan_dist_thresh(&self) -> f32 {
        self.scaled_pan_dist_thresh
    }

    /// When to pan general widgets (unhandled events) with the mouse
    #[inline]
    pub fn mouse_pan(&self) -> MousePan {
        self.mouse_pan
    }

    /// When to pan text fields with the mouse
    #[inline]
    pub fn mouse_text_pan(&self) -> MousePan {
        self.mouse_text_pan
    }

    /// Whether mouse clicks set keyboard navigation focus
    #[inline]
    pub fn mouse_nav_focus(&self) -> bool {
        self.mouse_nav_focus
    }

    /// Whether touchscreen events set keyboard navigation focus
    #[inline]
    pub fn touch_nav_focus(&self) -> bool {
        self.touch_nav_focus
    }

    /// Read shortcut config
    #[inline]
    pub fn shortcuts(&self) -> &Shortcuts {
        &self.shortcuts
    }
}

/// Other functions
impl Config {
    /// Has the config ever been updated?
    #[inline]
    pub fn is_dirty(&self) -> bool {
        false // current code never updates config
    }
}

/// When mouse-panning is enabled (click+drag to scroll)
///
/// For *text* objects, this may conflict with text selection, hence it is
/// recommended to require a modifier or disable this feature.
///
/// For non-text cases, this does not conflict with other event handlers since
/// panning is only possible when events are otherwise unused, thus `Always` is
/// acceptable (equivalent to touch scrolling).
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
#[cfg_attr(feature = "config", derive(Serialize, Deserialize))]
pub enum MousePan {
    /// Disable
    Never,
    /// Only enable when the Alt key is held
    WithAlt,
    /// Only enable when the Ctrl key is held
    WithCtrl,
    /// Always enabled
    Always,
}

impl MousePan {
    /// Is this enabled with the current modifiers?
    pub fn is_enabled_with(self, modifiers: ModifiersState) -> bool {
        match self {
            MousePan::Never => false,
            MousePan::WithAlt => modifiers.alt(),
            MousePan::WithCtrl => modifiers.ctrl(),
            MousePan::Always => true,
        }
    }
}

mod defaults {
    use super::MousePan;

    pub fn menu_delay_ns() -> u32 {
        250_000_000
    }
    pub fn touch_text_sel_delay_ns() -> u32 {
        1_000_000_000
    }
    pub fn pan_dist_thresh() -> f32 {
        2.1
    }
    pub fn mouse_pan() -> MousePan {
        MousePan::Always
    }
    pub fn mouse_text_pan() -> MousePan {
        #[cfg(target_os = "windows")]
        {
            MousePan::WithAlt
        }
        #[cfg(not(target_os = "windows"))]
        {
            MousePan::WithCtrl
        }
    }
    pub fn mouse_nav_focus() -> bool {
        true
    }
    pub fn touch_nav_focus() -> bool {
        true
    }
}
