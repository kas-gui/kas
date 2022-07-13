// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drivers for configuration types

use crate::view::driver;
use crate::{CheckButton, ComboBox, Spinner};
use kas::event::config::{Config, MousePan};
use kas::model::{SharedData, SharedRc};
use kas::prelude::*;

impl_scope! {
    /// A widget for viewing event config
    #[widget{
        layout = grid: {
            0, 0: "Menu delay:"; 1, 0: self.menu_delay; 2, 0: "ms";
            0, 1: "Touch-selection delay:"; 1, 1: self.touch_select_delay; 2, 1: "ms";
            0, 2: "Scroll flick timeout:"; 1, 2: self.scroll_flick_timeout; 2, 2: "ms";
            0, 3: "Scroll flick multiply:"; 1, 3: self.scroll_flick_mul;
            0, 4: "Scroll flick subtract:"; 1, 4: self.scroll_flick_sub;
            0, 5: "Pan distance threshold:"; 1, 5: self.pan_dist_thresh;
            0, 6: "Mouse pan:"; 1..3, 6: self.mouse_pan;
            0, 7: "Mouse text pan:"; 1..3, 7: self.mouse_text_pan;
            1..3, 8: self.mouse_nav_focus;
            1..3, 9: self.touch_nav_focus;
        };
    }]
    #[derive(Debug)]
    pub struct EventConfig {
        core: widget_core!(),
        #[widget] menu_delay: Spinner<u32>,
        #[widget] touch_select_delay: Spinner<u32>,
        #[widget] scroll_flick_timeout: Spinner<u32>,
        #[widget] scroll_flick_mul: Spinner<f32>,
        #[widget] scroll_flick_sub: Spinner<f32>,
        #[widget] pan_dist_thresh: Spinner<f32>,
        #[widget] mouse_pan: ComboBox<MousePan>,
        #[widget] mouse_text_pan: ComboBox<MousePan>,
        #[widget] mouse_nav_focus: CheckButton,
        #[widget] touch_nav_focus: CheckButton,
    }
}

impl driver::Driver<Config, SharedRc<Config>> for driver::DefaultView {
    type Widget = EventConfig;

    fn make(&self) -> Self::Widget {
        let mouse_pan = ComboBox::from([
            ("Never", MousePan::Never),
            ("With Alt key", MousePan::WithAlt),
            ("With Ctrl key", MousePan::WithCtrl),
            ("Always", MousePan::Always),
        ]);
        let mouse_text_pan = mouse_pan.clone();

        EventConfig {
            core: Default::default(),
            menu_delay: Spinner::new(0..=10_000, 50),
            touch_select_delay: Spinner::new(0..=10_000, 50),
            scroll_flick_timeout: Spinner::new(0..=1_000, 5),
            scroll_flick_mul: Spinner::new(0.0..=1.0, 0.0625),
            scroll_flick_sub: Spinner::new(0.0..=1.0e4, 10.0),
            pan_dist_thresh: Spinner::new(0.125..=10.0, 0.125),
            mouse_pan,
            mouse_text_pan,
            mouse_nav_focus: CheckButton::new("Mouse navigation focus"),
            touch_nav_focus: CheckButton::new("Touchscreen navigation focus"),
        }
    }

    fn set(&self, widget: &mut Self::Widget, data: &SharedRc<Config>, key: &()) -> TkAction {
        let data = data.get_cloned(key).unwrap();
        widget.menu_delay.set_value(data.menu_delay_ms)
            | widget
                .touch_select_delay
                .set_value(data.touch_select_delay_ms)
            | widget
                .scroll_flick_timeout
                .set_value(data.scroll_flick_timeout_ms)
            | widget.scroll_flick_mul.set_value(data.scroll_flick_mul)
            | widget.scroll_flick_sub.set_value(data.scroll_flick_sub)
            | widget.pan_dist_thresh.set_value(data.pan_dist_thresh)
            | widget.mouse_pan.set_active(data.mouse_pan as usize)
            | widget
                .mouse_text_pan
                .set_active(data.mouse_text_pan as usize)
            | widget.mouse_nav_focus.set_bool(data.mouse_nav_focus)
            | widget.touch_nav_focus.set_bool(data.touch_nav_focus)
    }
}
