// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context state

use linear_map::{LinearMap, set::LinearSet};
use smallvec::SmallVec;
use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
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
use crate::runner::{Platform, RunnerT, WindowDataErased};
use crate::theme::ThemeSize;
use crate::window::{PopupDescriptor, WindowId};
use crate::{ActionMoved, ConfigAction, HasId, Id, Node, WindowAction};
use key::PendingSelFocus;
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
    pub(crate) config: WindowConfig,
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
    access_keys: HashMap<Key, Id>,
    popups: SmallVec<[PopupState; 16]>,
    popup_removed: SmallVec<[(Id, WindowId); 16]>,
    time_updates: Vec<(Instant, Id, TimerHandle)>,
    frame_updates: LinearSet<(Id, TimerHandle)>,
    need_frame_update: bool,
    // Set of messages awaiting sending
    send_queue: VecDeque<(Id, Erased)>,
    // Set of futures of messages together with id of sending widget
    fut_messages: Vec<(Id, Pin<Box<dyn Future<Output = Erased>>>)>,
    pub(super) pending_send_targets: Vec<(TypeId, Id)>,
    // Widget requiring update
    pending_update: Option<Id>,
    // Optional new target for selection focus. bool is true if this also gains key focus.
    pending_sel_focus: Option<PendingSelFocus>,
    pending_nav_focus: PendingNavFocus,
    pub(crate) action: WindowAction,
    action_moved: ActionMoved,
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
            access_keys: Default::default(),
            popups: Default::default(),
            popup_removed: Default::default(),
            time_updates: vec![],
            frame_updates: Default::default(),
            need_frame_update: false,
            send_queue: Default::default(),
            pending_send_targets: vec![],
            fut_messages: vec![],
            pending_update: None,
            pending_sel_focus: None,
            pending_nav_focus: PendingNavFocus::None,
            action: WindowAction::empty(),
            action_moved: ActionMoved(false),
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
    pub(crate) fn full_configure(&mut self, sizer: &dyn ThemeSize, node: Node) {
        let id = Id::ROOT.make_child(self.window_id.get().cast());

        log::debug!(target: "kas_core::event", "full_configure of Window{id}");

        // These are recreated during configure:
        self.nav_fallback = None;

        let mut cx = ConfigCx::new(sizer, self);
        cx.configure(node, id);
        if *cx.resize {
            self.action |= WindowAction::RESIZE;
        }
        // Ignore cx.redraw: we can assume a redraw will happen
        self.action_moved = ActionMoved(true);
    }

    /// Construct a [`EventCx`] referring to this state
    ///
    /// Invokes the given closure on this [`EventCx`].
    #[inline]
    pub(crate) fn with<'a, F: FnOnce(&mut EventCx)>(
        &'a mut self,
        runner: &'a mut dyn RunnerT,
        theme: &'a dyn ThemeSize,
        window: &'a dyn WindowDataErased,
        f: F,
    ) {
        let mut cx = EventCx {
            runner,
            window,
            cx: ConfigCx::new(theme, self),
            target_is_disabled: false,
            last_child: None,
            scroll: Scroll::None,
        };
        f(&mut cx);
        if *cx.resize {
            self.action |= WindowAction::RESIZE;
        } else if cx.redraw {
            self.action |= WindowAction::REDRAW;
        }
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

    /// Access configuration data
    ///
    /// All widgets will be reconfigured if configuration data changes.
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
            self.redraw(&target);
            self.disabled.push(target);
        }
    }

    /// Notify that a widget must be redrawn
    #[inline]
    pub fn redraw(&mut self, id: impl HasId) {
        // NOTE: redraws are fast enough not to bother handling locally
        let _ = id;

        self.action |= WindowAction::REDRAW;
    }

    /// Redraw `id` if not `None`
    #[inline]
    pub(crate) fn opt_redraw(&mut self, id: Option<Id>) {
        if let Some(id) = id {
            self.redraw(id);
        }
    }

    /// Notify that widgets under self may have moved
    ///
    /// This updates the widget(s) under mouse and touch events.
    #[inline]
    pub fn region_moved(&mut self) {
        self.action_moved = ActionMoved(true);
    }

    /// Notify that a [`WindowAction`] should happen for the whole window
    #[inline]
    pub fn window_action(&mut self, action: impl Into<WindowAction>) {
        self.action |= action.into();
    }

    /// Request that the window be closed
    #[inline]
    pub fn close_own_window(&mut self) {
        self.action |= WindowAction::CLOSE;
    }

    /// Notify of an [`ActionMoved`]
    #[inline]
    pub fn action_moved(&mut self, action: ActionMoved) {
        self.action_moved |= action;
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
    runner: &'a mut dyn RunnerT,
    window: &'a dyn WindowDataErased,
    cx: ConfigCx<'a>,
    pub(crate) target_is_disabled: bool,
    last_child: Option<usize>,
    scroll: Scroll,
}

impl<'a> Deref for EventCx<'a> {
    type Target = ConfigCx<'a>;
    fn deref(&self) -> &Self::Target {
        &self.cx
    }
}
impl<'a> DerefMut for EventCx<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cx
    }
}

impl<'a> EventCx<'a> {
    /// Get a [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.runner.draw_shared()
    }

    /// Update configuration data
    #[inline]
    pub fn change_config(&mut self, msg: ConfigMsg) {
        let action = self.config.change_config(msg);
        if !action.is_empty() {
            self.runner.config_update(action);
        }
    }

    /// Update configuration data using a closure
    pub fn with_config(&mut self, f: impl FnOnce(&WindowConfig) -> ConfigAction) {
        let action = f(&self.config);
        if !action.is_empty() {
            self.runner.config_update(action);
        }
    }

    /// Terminate the GUI
    #[inline]
    pub fn exit(&mut self) {
        self.runner.exit();
    }
}
