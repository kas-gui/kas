// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drivers for configuration types

use crate::driver;
use crate::MaybeOwned;
use kas::event::config::{Config, MousePan};
use kas::model::SharedRc;
use kas::prelude::*;
use kas_widgets::{CheckButton, ComboBox, Spinner, TextButton};

#[derive(Clone, Debug)]
enum Msg {
    MenuDelay(u32),
    TouchSelectDelay(u32),
    ScrollFlickTimeout(u32),
    ScrollFlickMul(f32),
    ScrollFlickSub(f32),
    ScrollDistEm(f32),
    PanDistThresh(f32),
    MousePan(MousePan),
    MouseTextPan(MousePan),
    MouseNavFocus(bool),
    TouchNavFocus(bool),
    Reset,
}

impl_scope! {
    /// A widget for viewing event config
    #[widget{
        layout = grid: {
            0, 0: "Menu delay:"; 1, 0: self.menu_delay; 2, 0: "ms";
            0, 1: "Touch-selection delay:"; 1, 1: self.touch_select_delay; 2, 1: "ms";
            0, 2: "Scroll flick timeout:"; 1, 2: self.scroll_flick_timeout; 2, 2: "ms";
            0, 3: "Scroll flick multiply:"; 1, 3: self.scroll_flick_mul;
            0, 4: "Scroll flick subtract:"; 1, 4: self.scroll_flick_sub;
            0, 5: "Scroll wheel distance:"; 1, 5: self.scroll_dist_em; 2, 5: "em";
            0, 6: "Pan distance threshold:"; 1, 6: self.pan_dist_thresh;
            0, 7: "Mouse pan:"; 1..3, 7: self.mouse_pan;
            0, 8: "Mouse text pan:"; 1..3, 8: self.mouse_text_pan;
            1..3, 9: self.mouse_nav_focus;
            1..3, 10: self.touch_nav_focus;
            0, 11: "Restore default values:"; 1..3, 11: TextButton::new_msg("&Reset", Msg::Reset);
        };
    }]
    pub struct EventConfigWidget {
        core: widget_core!(),
        #[widget]
        menu_delay: Spinner<u32>,
        #[widget]
        touch_select_delay: Spinner<u32>,
        #[widget]
        scroll_flick_timeout: Spinner<u32>,
        #[widget]
        scroll_flick_mul: Spinner<f32>,
        #[widget]
        scroll_flick_sub: Spinner<f32>,
        #[widget]
        scroll_dist_em: Spinner<f32>,
        #[widget]
        pan_dist_thresh: Spinner<f32>,
        #[widget]
        mouse_pan: ComboBox<MousePan>,
        #[widget]
        mouse_text_pan: ComboBox<MousePan>,
        #[widget]
        mouse_nav_focus: CheckButton,
        #[widget]
        touch_nav_focus: CheckButton,
    }
}

/// Editor for [`kas::event::Config`](Config)
#[derive(Clone, Copy, Debug, Default)]
pub struct EventConfig;

impl driver::Driver<Config, SharedRc<Config>> for EventConfig {
    type Widget = EventConfigWidget;

    fn make(&self) -> Self::Widget {
        let pan_options = [
            ("&Never", MousePan::Never),
            ("With &Alt key", MousePan::WithAlt),
            ("With &Ctrl key", MousePan::WithCtrl),
            ("Alwa&ys", MousePan::Always),
        ];

        EventConfigWidget {
            core: Default::default(),
            menu_delay: Spinner::new(0..=5_000, 50).on_change(|mgr, v| mgr.push(Msg::MenuDelay(v))),
            touch_select_delay: Spinner::new(0..=5_000, 50)
                .on_change(|mgr, v| mgr.push(Msg::TouchSelectDelay(v))),
            scroll_flick_timeout: Spinner::new(0..=500, 5)
                .on_change(|mgr, v| mgr.push(Msg::ScrollFlickTimeout(v))),
            scroll_flick_mul: Spinner::new(0.0..=1.0, 0.03125)
                .on_change(|mgr, v| mgr.push(Msg::ScrollFlickMul(v))),
            scroll_flick_sub: Spinner::new(0.0..=1.0e4, 10.0)
                .on_change(|mgr, v| mgr.push(Msg::ScrollFlickSub(v))),
            scroll_dist_em: Spinner::new(0.125..=125.0, 0.125)
                .on_change(|mgr, v| mgr.push(Msg::ScrollDistEm(v))),
            pan_dist_thresh: Spinner::new(0.25..=25.0, 0.25)
                .on_change(|mgr, v| mgr.push(Msg::PanDistThresh(v))),
            mouse_pan: ComboBox::from(pan_options).on_select(|mgr, v| mgr.push(Msg::MousePan(v))),
            mouse_text_pan: ComboBox::from(pan_options)
                .on_select(|mgr, v| mgr.push(Msg::MouseTextPan(v))),
            mouse_nav_focus: CheckButton::new("&Mouse navigation focus")
                .on_toggle(|mgr, v| mgr.push(Msg::MouseNavFocus(v))),
            touch_nav_focus: CheckButton::new("&Touchscreen navigation focus")
                .on_toggle(|mgr, v| mgr.push(Msg::TouchNavFocus(v))),
        }
    }

    fn set_mo(&self, widget: &mut Self::Widget, _: &(), data: MaybeOwned<Config>) -> Action {
        widget.menu_delay.set_value(data.menu_delay_ms)
            | widget
                .touch_select_delay
                .set_value(data.touch_select_delay_ms)
            | widget
                .scroll_flick_timeout
                .set_value(data.scroll_flick_timeout_ms)
            | widget.scroll_flick_mul.set_value(data.scroll_flick_mul)
            | widget.scroll_flick_sub.set_value(data.scroll_flick_sub)
            | widget.scroll_dist_em.set_value(data.scroll_dist_em)
            | widget.pan_dist_thresh.set_value(data.pan_dist_thresh)
            | widget.mouse_pan.set_active(data.mouse_pan as usize)
            | widget
                .mouse_text_pan
                .set_active(data.mouse_text_pan as usize)
            | widget.mouse_nav_focus.set_bool(data.mouse_nav_focus)
            | widget.touch_nav_focus.set_bool(data.touch_nav_focus)
    }

    fn on_message(
        &self,
        mgr: &mut EventMgr,
        _: &mut Self::Widget,
        data: &SharedRc<Config>,
        _: &(),
    ) {
        if let Some(msg) = mgr.try_pop() {
            let mut data = data.borrow_mut(mgr);
            match msg {
                Msg::MenuDelay(v) => data.menu_delay_ms = v,
                Msg::TouchSelectDelay(v) => data.touch_select_delay_ms = v,
                Msg::ScrollFlickTimeout(v) => data.scroll_flick_timeout_ms = v,
                Msg::ScrollFlickMul(v) => data.scroll_flick_mul = v,
                Msg::ScrollFlickSub(v) => data.scroll_flick_sub = v,
                Msg::ScrollDistEm(v) => data.scroll_dist_em = v,
                Msg::PanDistThresh(v) => data.pan_dist_thresh = v,
                Msg::MousePan(v) => data.mouse_pan = v,
                Msg::MouseTextPan(v) => data.mouse_text_pan = v,
                Msg::MouseNavFocus(v) => data.mouse_nav_focus = v,
                Msg::TouchNavFocus(v) => data.touch_nav_focus = v,
                Msg::Reset => *data = Config::default(),
            }
            data.is_dirty = true;
        }
    }
}
