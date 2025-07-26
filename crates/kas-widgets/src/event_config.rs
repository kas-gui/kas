// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drivers for configuration types

use crate::{Button, CheckButton, ComboBox, SpinBox};
use kas::config::{ConfigMsg, EventConfigMsg, MousePan};
use kas::prelude::*;

#[impl_self]
mod EventConfig {
    /// A widget for configuring event config
    ///
    /// This only needs to be added to a UI to be functional.
    ///
    /// Changes take effect immediately. A "Reset" button restores all
    /// configuration to compiled (not saved) default values.
    /// TODO: support undo and/or revert to saved values.
    #[widget]
    #[layout(grid! {
        (0, 0) => "Hover delay:",
        (1, 0) => self.menu_delay,

        (0, 1) => "Menu delay:",
        (1, 1) => self.menu_delay,

        (0, 2) => "Touch-selection delay:",
        (1, 2) => self.touch_select_delay,

        (0, 3) => "Kinetic scrolling timeout:",
        (1, 3) => self.kinetic_timeout,

        (0, 4) => "Kinetic decay (relative):",
        (1, 4) => self.kinetic_decay_mul,

        (0, 5) => "Kinetic decay (absolute):",
        (1, 5) => self.kinetic_decay_sub,

        (0, 6) => "Kinetic decay when grabbed:",
        (1, 6) => self.kinetic_grab_sub,

        (0, 7) => "Scroll wheel distance:",
        (1, 7) => self.scroll_dist_em,

        (0, 8) => "Pan distance threshold:",
        (1, 8) => self.pan_dist_thresh,

        (0, 9) => "Mouse pan:",
        (1, 9) => self.mouse_pan,

        (0, 10) => "Mouse text pan:",
        (1, 10) => self.mouse_text_pan,

        (1, 11) => self.mouse_wheel_actions,

        (1, 12) => self.mouse_nav_focus,

        (1, 13) => self.touch_nav_focus,

        (0, 14) => "Restore default values:",
        (1, 14) => Button::label_msg("&Reset", EventConfigMsg::ResetToDefault),
    })]
    #[impl_default(EventConfig::new())]
    pub struct EventConfig {
        core: widget_core!(),
        #[widget]
        hover_delay: SpinBox<(), u32>,
        #[widget]
        menu_delay: SpinBox<(), u32>,
        #[widget]
        touch_select_delay: SpinBox<(), u32>,
        #[widget]
        kinetic_timeout: SpinBox<(), u32>,
        #[widget]
        kinetic_decay_mul: SpinBox<(), f32>,
        #[widget]
        kinetic_decay_sub: SpinBox<(), f32>,
        #[widget]
        kinetic_grab_sub: SpinBox<(), f32>,
        #[widget]
        scroll_dist_em: SpinBox<(), f32>,
        #[widget]
        pan_dist_thresh: SpinBox<(), f32>,
        #[widget]
        mouse_pan: ComboBox<(), MousePan>,
        #[widget]
        mouse_text_pan: ComboBox<(), MousePan>,
        #[widget]
        mouse_wheel_actions: CheckButton<()>,
        #[widget]
        mouse_nav_focus: CheckButton<()>,
        #[widget]
        touch_nav_focus: CheckButton<()>,
    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, _: &()) {
            if let Some(msg) = cx.try_pop() {
                cx.change_config(ConfigMsg::Event(msg));
            }
        }
    }

    impl Self {
        /// Construct an instance of the widget
        pub fn new() -> Self {
            let pan_options = [
                ("&Never", MousePan::Never),
                ("With &Alt key", MousePan::WithAlt),
                ("With &Ctrl key", MousePan::WithCtrl),
                ("Alwa&ys", MousePan::Always),
            ];

            EventConfig {
                core: Default::default(),
                hover_delay: SpinBox::new(0..=10_000, |cx, _| {
                    cx.config().base().event.menu_delay_ms
                })
                .with_step(100)
                .with_msg(EventConfigMsg::HoverDelay)
                .with_unit("ms"),
                menu_delay: SpinBox::new(0..=5_000, |cx, _| cx.config().base().event.menu_delay_ms)
                    .with_step(50)
                    .with_msg(EventConfigMsg::MenuDelay)
                    .with_unit("ms"),
                touch_select_delay: SpinBox::new(0..=5_000, |cx: &ConfigCx, _| {
                    cx.config().base().event.touch_select_delay_ms
                })
                .with_step(50)
                .with_msg(EventConfigMsg::TouchSelectDelay)
                .with_unit("ms"),
                kinetic_timeout: SpinBox::new(0..=500, |cx: &ConfigCx, _| {
                    cx.config().base().event.kinetic_timeout_ms
                })
                .with_step(5)
                .with_msg(EventConfigMsg::KineticTimeout)
                .with_unit("ms"),
                kinetic_decay_mul: SpinBox::new(0.0..=1.0, |cx: &ConfigCx, _| {
                    cx.config().base().event.kinetic_decay_mul
                })
                .with_step(0.0625)
                .with_msg(EventConfigMsg::KineticDecayMul),
                kinetic_decay_sub: SpinBox::new(0.0..=1.0e4, |cx: &ConfigCx, _| {
                    cx.config().base().event.kinetic_decay_sub
                })
                .with_step(10.0)
                .with_msg(EventConfigMsg::KineticDecaySub),
                kinetic_grab_sub: SpinBox::new(0.0..=1.0e4, |cx: &ConfigCx, _| {
                    cx.config().base().event.kinetic_grab_sub
                })
                .with_step(5.0)
                .with_msg(EventConfigMsg::KineticGrabSub),
                scroll_dist_em: SpinBox::new(0.125..=125.0, |cx: &ConfigCx, _| {
                    cx.config().base().event.scroll_dist_em
                })
                .with_step(0.125)
                .with_msg(EventConfigMsg::ScrollDistEm)
                .with_unit("em"),
                pan_dist_thresh: SpinBox::new(0.25..=25.0, |cx: &ConfigCx, _| {
                    cx.config().base().event.pan_dist_thresh
                })
                .with_step(0.25)
                .with_msg(EventConfigMsg::PanDistThresh),
                mouse_pan: ComboBox::new_msg(
                    pan_options,
                    |cx: &ConfigCx, _| cx.config().base().event.mouse_pan,
                    EventConfigMsg::MousePan,
                ),
                mouse_text_pan: ComboBox::new_msg(
                    pan_options,
                    |cx: &ConfigCx, _| cx.config().base().event.mouse_text_pan,
                    EventConfigMsg::MouseTextPan,
                ),
                mouse_wheel_actions: CheckButton::new_msg(
                    "Mouse &wheel actions",
                    |cx: &ConfigCx, _| cx.config().base().event.mouse_wheel_actions,
                    EventConfigMsg::MouseWheelActions,
                ),
                mouse_nav_focus: CheckButton::new_msg(
                    "&Mouse navigation focus",
                    |cx: &ConfigCx, _| cx.config().base().event.mouse_nav_focus,
                    EventConfigMsg::MouseNavFocus,
                ),
                touch_nav_focus: CheckButton::new_msg(
                    "&Touchscreen navigation focus",
                    |cx: &ConfigCx, _| cx.config().base().event.touch_nav_focus,
                    EventConfigMsg::TouchNavFocus,
                ),
            }
        }
    }
}
