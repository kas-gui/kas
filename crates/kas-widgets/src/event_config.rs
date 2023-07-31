// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drivers for configuration types

use crate::{adapt::AdaptWidgetAny, CheckButton, ComboBox, Spinner, TextButton};
use kas::event::config::{ChangeConfig, MousePan};
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
            (1..3, 11) => TextButton::new_msg("&Reset", ChangeConfig::ResetToDefault).map_any(),
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
        fn handle_messages(&mut self, _: &(), mgr: &mut EventMgr) {
            if let Some(msg) = mgr.try_pop() {
                mgr.change_config(msg);
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
                menu_delay: Spinner::new(0..=5_000, |cx, _| cx.config().borrow().menu_delay_ms)
                    .with_step(50)
                    .on_change(|mgr, v| mgr.push(ChangeConfig::MenuDelay(v))),
                touch_select_delay: Spinner::new(0..=5_000, |cx: &ConfigMgr, _| cx.config().borrow().touch_select_delay_ms)
                    .with_step(50)
                    .on_change(|mgr, v| mgr.push(ChangeConfig::TouchSelectDelay(v))),
                scroll_flick_timeout: Spinner::new(0..=500, |cx: &ConfigMgr, _| cx.config().borrow().scroll_flick_timeout_ms)
                    .with_step(5)
                    .on_change(|mgr, v| mgr.push(ChangeConfig::ScrollFlickTimeout(v))),
                scroll_flick_mul: Spinner::new(0.0..=1.0, |cx: &ConfigMgr, _| cx.config().borrow().scroll_flick_mul)
                    .with_step(0.0625)
                    .on_change(|mgr, v| mgr.push(ChangeConfig::ScrollFlickMul(v))),
                scroll_flick_sub: Spinner::new(0.0..=1.0e4, |cx: &ConfigMgr, _| cx.config().borrow().scroll_flick_sub)
                    .with_step(10.0)
                    .on_change(|mgr, v| mgr.push(ChangeConfig::ScrollFlickSub(v))),
                scroll_dist_em: Spinner::new(0.125..=125.0, |cx: &ConfigMgr, _| cx.config().borrow().scroll_dist_em)
                    .with_step(0.125)
                    .on_change(|mgr, v| mgr.push(ChangeConfig::ScrollDistEm(v))),
                pan_dist_thresh: Spinner::new(0.25..=25.0, |cx: &ConfigMgr, _| cx.config().borrow().pan_dist_thresh)
                    .with_step(0.25)
                    .on_change(|mgr, v| mgr.push(ChangeConfig::PanDistThresh(v))),
                mouse_pan: ComboBox::new(pan_options, |cx: &ConfigMgr, _| cx.config().borrow().mouse_pan)
                    .on_select(|mgr, v| mgr.push(ChangeConfig::MousePan(v))),
                mouse_text_pan: ComboBox::new(pan_options, |cx: &ConfigMgr, _| cx.config().borrow().mouse_text_pan)
                    .on_select(|mgr, v| mgr.push(ChangeConfig::MouseTextPan(v))),
                mouse_nav_focus: CheckButton::new("&Mouse navigation focus", |cx: &ConfigMgr, _| cx.config().borrow().mouse_nav_focus)
                    .on_toggle(|mgr, _, v| mgr.push(ChangeConfig::MouseNavFocus(v))),
                touch_nav_focus: CheckButton::new("&Touchscreen navigation focus", |cx: &ConfigMgr, _| cx.config().borrow().touch_nav_focus)
                    .on_toggle(|mgr, _, v| mgr.push(ChangeConfig::TouchNavFocus(v))),
            }
        }
    }
}
