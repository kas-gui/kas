// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling configuration

use crate::Action;
use crate::cast::{Cast, CastFloat};
use crate::event::ModifiersState;
use crate::geom::Offset;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::cell::Ref;
use std::time::Duration;

/// A message which may be used to update [`EventConfig`]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EventConfigMsg {
    MenuDelay(u32),
    TouchSelectDelay(u32),
    KineticTimeout(u32),
    KineticDecayMul(f32),
    KineticDecaySub(f32),
    KineticGrabSub(f32),
    ScrollDistEm(f32),
    PanDistThresh(f32),
    MousePan(MousePan),
    MouseTextPan(MousePan),
    MouseWheelActions(bool),
    MouseNavFocus(bool),
    TouchNavFocus(bool),
    /// Reset all config values to default (not saved) values
    ResetToDefault,
}

/// Event handling configuration
///
/// This is serializable (using `feature = "serde"`) with the following fields:
///
/// > `menu_delay_ms`: `u32` (milliseconds) \
/// > `touch_select_delay_ms`: `u32` (milliseconds) \
/// > `kinetic_timeout_ms`: `u32` (milliseconds) \
/// > `kinetic_decay_mul`: `f32` (unitless, applied each second) \
/// > `kinetic_decay_sub`: `f32` (pixels per second) \
/// > `kinetic_grab_sub`: `f32` (pixels per second) \
/// > `pan_dist_thresh`: `f32` (pixels) \
/// > `mouse_pan`: [`MousePan`] \
/// > `mouse_text_pan`: [`MousePan`] \
/// > `mouse_nav_focus`: `bool` \
/// > `touch_nav_focus`: `bool`
///
/// For descriptions of configuration effects, see [`EventWindowConfig`] methods.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EventConfig {
    #[cfg_attr(feature = "serde", serde(default = "defaults::menu_delay_ms"))]
    pub menu_delay_ms: u32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::touch_select_delay_ms"))]
    pub touch_select_delay_ms: u32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::kinetic_timeout_ms"))]
    pub kinetic_timeout_ms: u32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::kinetic_decay_mul"))]
    pub kinetic_decay_mul: f32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::kinetic_decay_sub"))]
    pub kinetic_decay_sub: f32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::kinetic_grab_sub"))]
    pub kinetic_grab_sub: f32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::scroll_dist_em"))]
    pub scroll_dist_em: f32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::pan_dist_thresh"))]
    pub pan_dist_thresh: f32,

    #[cfg_attr(feature = "serde", serde(default = "defaults::mouse_pan"))]
    pub mouse_pan: MousePan,
    #[cfg_attr(feature = "serde", serde(default = "defaults::mouse_text_pan"))]
    pub mouse_text_pan: MousePan,

    #[cfg_attr(feature = "serde", serde(default = "defaults::mouse_wheel_actions"))]
    pub mouse_wheel_actions: bool,

    #[cfg_attr(feature = "serde", serde(default = "defaults::mouse_nav_focus"))]
    pub mouse_nav_focus: bool,
    #[cfg_attr(feature = "serde", serde(default = "defaults::touch_nav_focus"))]
    pub touch_nav_focus: bool,
}

impl Default for EventConfig {
    fn default() -> Self {
        EventConfig {
            menu_delay_ms: defaults::menu_delay_ms(),
            touch_select_delay_ms: defaults::touch_select_delay_ms(),
            kinetic_timeout_ms: defaults::kinetic_timeout_ms(),
            kinetic_decay_mul: defaults::kinetic_decay_mul(),
            kinetic_decay_sub: defaults::kinetic_decay_sub(),
            kinetic_grab_sub: defaults::kinetic_grab_sub(),
            scroll_dist_em: defaults::scroll_dist_em(),
            pan_dist_thresh: defaults::pan_dist_thresh(),
            mouse_pan: defaults::mouse_pan(),
            mouse_text_pan: defaults::mouse_text_pan(),
            mouse_wheel_actions: defaults::mouse_wheel_actions(),
            mouse_nav_focus: defaults::mouse_nav_focus(),
            touch_nav_focus: defaults::touch_nav_focus(),
        }
    }
}

impl EventConfig {
    pub(super) fn change_config(&mut self, msg: EventConfigMsg) -> Action {
        match msg {
            EventConfigMsg::MenuDelay(v) => self.menu_delay_ms = v,
            EventConfigMsg::TouchSelectDelay(v) => self.touch_select_delay_ms = v,
            EventConfigMsg::KineticTimeout(v) => self.kinetic_timeout_ms = v,
            EventConfigMsg::KineticDecayMul(v) => self.kinetic_decay_mul = v,
            EventConfigMsg::KineticDecaySub(v) => self.kinetic_decay_sub = v,
            EventConfigMsg::KineticGrabSub(v) => self.kinetic_grab_sub = v,
            EventConfigMsg::ScrollDistEm(v) => self.scroll_dist_em = v,
            EventConfigMsg::PanDistThresh(v) => self.pan_dist_thresh = v,
            EventConfigMsg::MousePan(v) => self.mouse_pan = v,
            EventConfigMsg::MouseTextPan(v) => self.mouse_text_pan = v,
            EventConfigMsg::MouseWheelActions(v) => self.mouse_wheel_actions = v,
            EventConfigMsg::MouseNavFocus(v) => self.mouse_nav_focus = v,
            EventConfigMsg::TouchNavFocus(v) => self.touch_nav_focus = v,
            EventConfigMsg::ResetToDefault => *self = EventConfig::default(),
        }

        Action::EVENT_CONFIG
    }
}

/// Accessor to event configuration
///
/// This is a helper to read event configuration, adapted for the current
/// application and window scale.
#[derive(Clone, Debug)]
pub struct EventWindowConfig<'a>(pub(super) &'a super::WindowConfig);

impl<'a> EventWindowConfig<'a> {
    /// Access base (unscaled) event [`EventConfig`]
    #[inline]
    pub fn base(&self) -> Ref<'_, EventConfig> {
        Ref::map(self.0.config.borrow(), |c| &c.event)
    }

    /// Delay before opening/closing menus on mouse over
    #[inline]
    pub fn menu_delay(&self) -> Duration {
        Duration::from_millis(self.base().menu_delay_ms.cast())
    }

    /// Delay before switching from panning to (text) selection mode
    #[inline]
    pub fn touch_select_delay(&self) -> Duration {
        Duration::from_millis(self.base().touch_select_delay_ms.cast())
    }

    /// Controls activation of kinetic scrolling
    ///
    /// This is the maximum time between the last press-movement and final
    /// release to activate kinetic scrolling mode. The last few `PressMove`
    /// events within this time window are used to calculate the initial speed.
    #[inline]
    pub fn kinetic_timeout(&self) -> Duration {
        Duration::from_millis(self.base().kinetic_timeout_ms.cast())
    }

    /// Kinetic scrolling decay: `(mul, sub)`
    ///
    /// The `mul` factor describes exponential decay: effectively, velocity is
    /// multiplied by `mul` every second. This is the dominant decay factor at
    /// high speeds; `mul = 1.0` implies no decay while `mul = 0.0` implies an
    /// instant stop.
    ///
    /// The `sub` factor describes linear decay: effectively, speed is reduced
    /// by `sub` every second. This is the dominant decay factor at low speeds.
    /// Units are pixels/second (output is adjusted for the window's scale factor).
    #[inline]
    pub fn kinetic_decay(&self) -> (f32, f32) {
        (self.base().kinetic_decay_mul, self.0.kinetic_decay_sub)
    }

    /// Kinetic scrolling decay while grabbed: `sub`
    ///
    /// When a kinetic-scrolling element is grabbed while in motion, speed
    /// relative to the grab velocity is reduced by this value each second.
    /// This is applied in addition to the usual velocity decay (see
    /// [`Self::kinetic_decay`]).
    #[inline]
    pub fn kinetic_grab_sub(&self) -> f32 {
        self.base().kinetic_grab_sub
    }

    /// Get distance in pixels to scroll due to mouse wheel
    ///
    /// Calculates scroll distance from `(horiz, vert)` lines.
    pub fn scroll_distance(&self, lines: (f32, f32)) -> Offset {
        let x = (self.0.scroll_dist * lines.0).cast_nearest();
        let y = (self.0.scroll_dist * lines.1).cast_nearest();
        Offset(x, y)
    }

    /// Drag distance threshold before panning (scrolling) starts
    ///
    /// When the distance moved is greater than this threshold, panning should
    /// start; otherwise the system should wait for the text-selection timer.
    /// We currently recommend the L-inf distance metric (max of abs of values).
    ///
    /// Units are pixels (output is adjusted for the window's scale factor).
    #[inline]
    pub fn pan_dist_thresh(&self) -> f32 {
        self.0.pan_dist_thresh
    }

    /// Whether the mouse wheel may trigger actions such as switching to the next item in a list
    #[inline]
    pub fn mouse_wheel_actions(&self) -> bool {
        self.base().mouse_wheel_actions
    }

    /// When to pan general widgets (unhandled events) with the mouse
    #[inline]
    pub fn mouse_pan(&self) -> MousePan {
        self.base().mouse_pan
    }

    /// When to pan text fields with the mouse
    #[inline]
    pub fn mouse_text_pan(&self) -> MousePan {
        self.base().mouse_text_pan
    }

    /// Whether mouse clicks set keyboard navigation focus
    #[inline]
    pub fn mouse_nav_focus(&self) -> bool {
        self.0.nav_focus && self.base().mouse_nav_focus
    }

    /// Whether touchscreen events set keyboard navigation focus
    #[inline]
    pub fn touch_nav_focus(&self) -> bool {
        self.0.nav_focus && self.base().touch_nav_focus
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
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(num_enum::IntoPrimitive, num_enum::TryFromPrimitive)]
#[repr(u8)]
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
            MousePan::WithAlt => modifiers.alt_key(),
            MousePan::WithCtrl => modifiers.control_key(),
            MousePan::Always => true,
        }
    }
}

mod defaults {
    use super::MousePan;

    pub fn menu_delay_ms() -> u32 {
        250
    }
    pub fn touch_select_delay_ms() -> u32 {
        1000
    }
    pub fn kinetic_timeout_ms() -> u32 {
        50
    }
    pub fn kinetic_decay_mul() -> f32 {
        0.625
    }
    pub fn kinetic_decay_sub() -> f32 {
        200.0
    }
    pub fn kinetic_grab_sub() -> f32 {
        10000.0
    }
    pub fn scroll_dist_em() -> f32 {
        4.5
    }
    pub fn pan_dist_thresh() -> f32 {
        5.0
    }
    pub fn mouse_wheel_actions() -> bool {
        true
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
