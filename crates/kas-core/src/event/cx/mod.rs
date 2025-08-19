// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context state

use linear_map::{LinearMap, set::LinearSet};
use smallvec::SmallVec;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::time::Instant;

use super::*;
use crate::cast::{Cast, Conv};
use crate::config::{ConfigMsg, WindowConfig};
use crate::draw::DrawShared;
use crate::geom::{Offset, Rect, Vec2};
use crate::messages::Erased;
use crate::runner::{MessageStack, Platform, RunnerT, WindowDataErased};
use crate::theme::{SizeCx, ThemeSize};
use crate::window::{PopupDescriptor, Window, WindowId};
use crate::{Action, HasId, Id, Node};
use key::{AccessLayer, PendingSelFocus};
use nav::PendingNavFocus;

#[cfg(feature = "accesskit")] mod accessibility;
mod key;
mod nav;
mod press;
mod send;
mod timer;
mod window;

pub use nav::NavAdvance;
pub use press::{GrabBuilder, GrabMode, Press, PressSource, PressStart};
pub(crate) use press::{Mouse, Touch};
pub use timer::TimerHandle;

struct PopupState {
    id: WindowId,
    desc: PopupDescriptor,
    old_nav_focus: Option<Id>,
    is_sized: bool,
}

/// Event context state
///
/// This struct encapsulates window-specific event-handling state and handling.
/// Most operations are only available via a [`EventCx`] handle, though some
/// are available on this struct.
///
/// Besides event handling, this struct also configures widgets.
///
/// Some methods are intended only for usage by graphics and platform backends
/// and are hidden from generated documentation unless the `internal_doc`
/// feature is enabled. Event handling assumes [winit].
///
/// [winit]: https://github.com/rust-windowing/winit
//
// Note that the most frequent usage of fields is to check highlighting states
// for each widget during drawing. Most fields contain only a few values, hence
// `SmallVec` is used to keep contents in local memory.
pub struct EventState {
    pub(crate) window_id: WindowId,
    config: WindowConfig,
    platform: Platform,
    disabled: Vec<Id>,
    window_has_focus: bool,
    #[cfg(feature = "accesskit")]
    accesskit_is_enabled: bool,
    modifiers: ModifiersState,
    /// Key (and IME) focus is on same widget as sel_focus; otherwise its value is ignored
    key_focus: bool,
    ime: Option<ImePurpose>,
    old_ime_target: Option<Id>,
    /// Rect is cursor area in sel_focus's coordinate space if size != ZERO
    ime_cursor_area: Rect,
    last_ime_rect: Rect,
    sel_focus: Option<Id>,
    nav_focus: Option<Id>,
    nav_fallback: Option<Id>,
    key_depress: LinearMap<PhysicalKey, Id>,
    mouse: Mouse,
    touch: Touch,
    access_layers: BTreeMap<Id, AccessLayer>,
    popups: SmallVec<[PopupState; 16]>,
    popup_removed: SmallVec<[(Id, WindowId); 16]>,
    time_updates: Vec<(Instant, Id, TimerHandle)>,
    frame_updates: LinearSet<(Id, TimerHandle)>,
    need_frame_update: bool,
    // Set of messages awaiting sending
    send_queue: VecDeque<(Id, Erased)>,
    // Set of futures of messages together with id of sending widget
    fut_messages: Vec<(Id, Pin<Box<dyn Future<Output = Erased>>>)>,
    // Widget requiring update
    pending_update: Option<Id>,
    // Optional new target for selection focus. bool is true if this also gains key focus.
    pending_sel_focus: Option<PendingSelFocus>,
    pending_nav_focus: PendingNavFocus,
    pub(crate) action: Action,
}

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

    /// Clear all focus and grabs on `target`
    fn cancel_event_focus(&mut self, target: &Id) {
        self.clear_sel_socus_on(target);
        self.clear_nav_focus_on(target);
        self.mouse.cancel_event_focus(target);
        self.touch.cancel_event_focus(target);
    }

    /// Check whether a widget is disabled
    ///
    /// A widget is disabled if any ancestor is.
    #[inline]
    pub fn is_disabled(&self, w_id: &Id) -> bool {
        // TODO(opt): we should be able to use binary search here
        for id in &self.disabled {
            if id.is_ancestor_of(w_id) {
                return true;
            }
        }
        false
    }

    /// Access event-handling configuration
    #[inline]
    pub fn config(&self) -> &WindowConfig {
        &self.config
    }

    /// Is mouse panning enabled?
    #[inline]
    pub fn config_enable_pan(&self, source: PressSource) -> bool {
        source.is_touch()
            || source.is_primary()
                && self
                    .config
                    .event()
                    .mouse_pan()
                    .is_enabled_with(self.modifiers())
    }

    /// Is mouse text panning enabled?
    #[inline]
    pub fn config_enable_mouse_text_pan(&self) -> bool {
        self.config
            .event()
            .mouse_text_pan()
            .is_enabled_with(self.modifiers())
    }

    /// Test pan threshold against config, adjusted for scale factor
    ///
    /// Returns true when `dist` is large enough to switch to pan mode.
    #[inline]
    pub fn config_test_pan_thresh(&self, dist: Offset) -> bool {
        Vec2::conv(dist).abs().max_comp() >= self.config.event().pan_dist_thresh()
    }

    /// Update event configuration
    #[inline]
    pub fn change_config(&mut self, msg: ConfigMsg) {
        self.action |= self.config.change_config(msg);
    }

    /// Set/unset a widget as disabled
    ///
    /// Disabled status applies to all descendants and blocks reception of
    /// events ([`Unused`] is returned automatically when the
    /// recipient or any ancestor is disabled).
    ///
    /// Disabling a widget clears navigation, selection and key focus when the
    /// target is disabled, and also cancels press/pan grabs.
    pub fn set_disabled(&mut self, target: Id, disable: bool) {
        if disable {
            self.cancel_event_focus(&target);
        }

        for (i, id) in self.disabled.iter().enumerate() {
            if target == id {
                if !disable {
                    self.redraw(target);
                    self.disabled.remove(i);
                }
                return;
            }
        }
        if disable {
            self.action(&target, Action::REDRAW);
            self.disabled.push(target);
        }
    }

    /// Notify that a widget must be redrawn
    ///
    /// This is equivalent to calling [`Self::action`] with [`Action::REDRAW`].
    #[inline]
    pub fn redraw(&mut self, id: impl HasId) {
        self.action(id, Action::REDRAW);
    }

    /// Notify that a widget must be resized
    ///
    /// This is equivalent to calling [`Self::action`] with [`Action::RESIZE`].
    #[inline]
    pub fn resize(&mut self, id: impl HasId) {
        self.action(id, Action::RESIZE);
    }

    /// Notify that widgets under self may have moved
    #[inline]
    pub fn region_moved(&mut self) {
        // Do not take id: this always applies to the whole window
        self.action |= Action::REGION_MOVED;
    }

    /// Terminate the GUI
    #[inline]
    pub fn exit(&mut self) {
        self.send(Id::ROOT, Command::Exit);
    }

    /// Notify that an [`Action`] should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    #[inline]
    pub fn action(&mut self, id: impl HasId, action: Action) {
        fn inner(cx: &mut EventState, id: Id, mut action: Action) {
            if action.contains(Action::UPDATE) {
                cx.request_update(id);
                action.remove(Action::UPDATE);
            }

            // TODO(opt): handle sub-tree SCROLLED. This is probably equivalent to using `_replay` without a message but with `scroll = Scroll::Scrolled`.
            // TODO(opt): handle sub-tree SET_RECT and RESIZE.
            // NOTE: our draw system is incompatible with partial redraws, and
            // in any case redrawing is extremely fast.

            cx.action |= action;
        }
        inner(self, id.has_id(), action)
    }

    /// Pass an [action](Self::action) given some `id`
    #[inline]
    pub(crate) fn opt_action(&mut self, id: Option<Id>, action: Action) {
        if let Some(id) = id {
            self.action(id, action);
        }
    }

    /// Notify that an [`Action`] should happen for the whole window
    ///
    /// Using [`Self::action`] with a widget `id` instead of this method is
    /// potentially more efficient (supports future optimisations), but not
    /// always possible.
    #[inline]
    pub fn window_action(&mut self, action: Action) {
        self.action |= action;
    }

    /// Request update to widget `id`
    ///
    /// This will call [`Events::update`] on `id`.
    pub fn request_update(&mut self, id: Id) {
        self.pending_update = if let Some(id2) = self.pending_update.take() {
            Some(id.common_ancestor(&id2))
        } else {
            Some(id)
        };
    }
}

/// Event handling context
///
/// `EventCx` and [`EventState`] (available via [`Deref`]) support various
/// event management and event-handling state querying operations.
#[must_use]
pub struct EventCx<'a> {
    state: &'a mut EventState,
    runner: &'a mut dyn RunnerT,
    window: &'a dyn WindowDataErased,
    messages: &'a mut MessageStack,
    pub(crate) target_is_disabled: bool,
    last_child: Option<usize>,
    scroll: Scroll,
}

impl<'a> Deref for EventCx<'a> {
    type Target = EventState;
    fn deref(&self) -> &Self::Target {
        self.state
    }
}
impl<'a> DerefMut for EventCx<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl<'a> EventCx<'a> {
    /// Configure a widget
    ///
    /// This is a shortcut to [`ConfigCx::configure`].
    #[inline]
    pub fn configure(&mut self, mut widget: Node<'_>, id: Id) {
        widget._configure(&mut self.config_cx(), id);
    }

    /// Update a widget
    ///
    /// [`Events::update`] will be called recursively on each child and finally
    /// `self`. If a widget stores state which it passes to children as input
    /// data, it should call this (or [`ConfigCx::update`]) after mutating the state.
    #[inline]
    pub fn update(&mut self, mut widget: Node<'_>) {
        widget._update(&mut self.config_cx());
    }

    /// Get a [`SizeCx`]
    ///
    /// Warning: sizes are calculated using the window's current scale factor.
    /// This may change, even without user action, since some platforms
    /// always initialize windows with scale factor 1.
    /// See also notes on [`Events::configure`].
    pub fn size_cx(&self) -> SizeCx<'_> {
        SizeCx::new(self.window.theme_size())
    }

    /// Get a [`ConfigCx`]
    pub fn config_cx(&mut self) -> ConfigCx<'_> {
        let size = self.window.theme_size();
        ConfigCx::new(size, self.state)
    }

    /// Get a [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.runner.draw_shared()
    }
}
