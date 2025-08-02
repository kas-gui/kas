// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — platform API

use super::*;
use crate::theme::ThemeSize;
use crate::window::Window;

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
impl EventState {
    /// Construct per-window event state
    #[inline]
    pub(crate) fn new(window_id: WindowId, config: WindowConfig, platform: Platform) -> Self {
        EventState {
            window_id,
            config,
            platform,
            disabled: vec![],
            window_has_focus: false,
            #[cfg(feature = "accesskit")]
            accesskit_is_enabled: false,
            modifiers: ModifiersState::empty(),
            key_focus: false,
            ime: None,
            old_ime_target: None,
            ime_cursor_area: Rect::ZERO,
            last_ime_rect: Rect::ZERO,
            sel_focus: None,
            nav_focus: None,
            nav_fallback: None,
            key_depress: Default::default(),
            mouse: Default::default(),
            touch: Default::default(),
            access_layers: Default::default(),
            popups: Default::default(),
            popup_removed: Default::default(),
            time_updates: vec![],
            frame_updates: Default::default(),
            need_frame_update: false,
            send_queue: Default::default(),
            fut_messages: vec![],
            pending_update: None,
            pending_sel_focus: None,
            pending_nav_focus: PendingNavFocus::None,
            action: Action::empty(),
        }
    }

    /// Update scale factor
    pub(crate) fn update_config(&mut self, scale_factor: f32) {
        self.config.update(scale_factor);
    }

    /// Configure a widget tree
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    ///
    /// This method calls [`ConfigCx::configure`] in order to assign
    /// [`Id`] identifiers and call widgets' [`Events::configure`]
    /// method. Additionally, it updates the [`EventState`] to account for
    /// renamed and removed widgets.
    pub(crate) fn full_configure<A>(
        &mut self,
        sizer: &dyn ThemeSize,
        win: &mut Window<A>,
        data: &A,
    ) {
        let id = Id::ROOT.make_child(self.window_id.get().cast());

        log::debug!(target: "kas_core::event", "full_configure of Window{id}");

        // These are recreated during configure:
        self.access_layers.clear();
        self.nav_fallback = None;

        self.new_access_layer(id.clone(), false);

        ConfigCx::new(sizer, self).configure(win.as_node(data), id);
        self.action |= Action::REGION_MOVED;
    }

    /// Construct a [`EventCx`] referring to this state
    ///
    /// Invokes the given closure on this [`EventCx`].
    #[inline]
    pub(crate) fn with<'a, F: FnOnce(&mut EventCx)>(
        &'a mut self,
        runner: &'a mut dyn RunnerT,
        window: &'a dyn WindowDataErased,
        messages: &'a mut MessageStack,
        f: F,
    ) {
        let mut cx = EventCx {
            state: self,
            runner,
            window,
            messages,
            target_is_disabled: false,
            last_child: None,
            scroll: Scroll::None,
        };
        f(&mut cx);
    }
}
