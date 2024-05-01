// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drivers for configuration types

use crate::{Button, CheckButton, ComboBox, Spinner};
use kas::config::{ConfigMsg, EventConfigMsg, MousePan};
use kas::prelude::*;

impl_scope! {
    /// A widget for configuring event config
    ///
    /// This only needs to be added to a UI to be functional.
    ///
    /// Changes take effect immediately. A "Reset" button restores all
    /// configuration to compiled (not saved) default values.
    /// TODO: support undo and/or revert to saved values.
    #[widget{
        Data = ();
        layout = grid! {
            (0, 0) => "Menu delay:",
            (1, 0) => self.menu_delay,
            (2, 0) => "ms",

            (0, 1) => "Touch-selection delay:",
            (1, 1) => self.touch_select_delay,
            (2, 1) => "ms",

            (0, 2) => "Scroll flick timeout:",
            (1, 2) => self.scroll_flick_timeout,
            (2, 2) => "ms",

            (0, 3) => "Scroll flick multiply:",
            (1, 3) => self.scroll_flick_mul,

            (0, 4) => "Scroll flick subtract:",
            (1, 4) => self.scroll_flick_sub,

            (0, 5) => "Scroll wheel distance:",
            (1, 5) => self.scroll_dist_em, (2, 5) => "em",

            (0, 6) => "Pan distance threshold:",
            (1, 6) => self.pan_dist_thresh,

            (0, 7) => "Mouse pan:",
            (1..3, 7) => self.mouse_pan,

            (0, 8) => "Mouse text pan:",
            (1..3, 8) => self.mouse_text_pan,

            (1..3, 9) => self.mouse_nav_focus,

            (1..3, 10) => self.touch_nav_focus,

            (0, 11) => "Restore default values:",
            (1..3, 11) => Button::label_msg("&Reset", EventConfigMsg::ResetToDefault),
        };
    }]
    #[impl_default(EventConfig::new())]
    pub struct EventConfig {
        core: widget_core!(),
        #[widget]
        menu_delay: Spinner<(), u32>,
        #[widget]
        touch_select_delay: Spinner<(), u32>,
        #[widget]
        scroll_flick_timeout: Spinner<(), u32>,
        #[widget]
        scroll_flick_mul: Spinner<(), f32>,
        #[widget]
        scroll_flick_sub: Spinner<(), f32>,
        #[widget]
        scroll_dist_em: Spinner<(), f32>,
        #[widget]
        pan_dist_thresh: Spinner<(), f32>,
        #[widget]
        mouse_pan: ComboBox<(), MousePan>,
        #[widget]
        mouse_text_pan: ComboBox<(), MousePan>,
        #[widget]
        mouse_nav_focus: CheckButton<()>,
        #[widget]
        touch_nav_focus: CheckButton<()>,
    }

    impl Events for Self {
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
                menu_delay: Spinner::new(0..=5_000, |cx, _| cx.config().base().event.menu_delay_ms)
                    .with_step(50)
                    .with_msg(EventConfigMsg::MenuDelay),
                touch_select_delay: Spinner::new(0..=5_000, |cx: &ConfigCx, _| cx.config().base().event.touch_select_delay_ms)
                    .with_step(50)
                    .with_msg(EventConfigMsg::TouchSelectDelay),
                scroll_flick_timeout: Spinner::new(0..=500, |cx: &ConfigCx, _| cx.config().base().event.scroll_flick_timeout_ms)
                    .with_step(5)
                    .with_msg(EventConfigMsg::ScrollFlickTimeout),
                scroll_flick_mul: Spinner::new(0.0..=1.0, |cx: &ConfigCx, _| cx.config().base().event.scroll_flick_mul)
                    .with_step(0.0625)
                    .with_msg(EventConfigMsg::ScrollFlickMul),
                scroll_flick_sub: Spinner::new(0.0..=1.0e4, |cx: &ConfigCx, _| cx.config().base().event.scroll_flick_sub)
                    .with_step(10.0)
                    .with_msg(EventConfigMsg::ScrollFlickSub),
                scroll_dist_em: Spinner::new(0.125..=125.0, |cx: &ConfigCx, _| cx.config().base().event.scroll_dist_em)
                    .with_step(0.125)
                    .with_msg(EventConfigMsg::ScrollDistEm),
                pan_dist_thresh: Spinner::new(0.25..=25.0, |cx: &ConfigCx, _| cx.config().base().event.pan_dist_thresh)
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
