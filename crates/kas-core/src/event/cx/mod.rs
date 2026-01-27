// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context state

use linear_map::{LinearMap, set::LinearSet};
use smallvec::SmallVec;
use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
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
#[allow(unused)] use crate::theme::SizeCx;
use crate::theme::ThemeSize;
use crate::window::{PopupDescriptor, WindowId};
use crate::{ActionClose, ActionMoved, ActionRedraw, ActionResize, ConfigAction, HasId, Id, Node};
use key::{Input, PendingSelFocus};
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
    ime_is_enabled: bool,
    old_ime_target: Option<Id>,
    /// Rect is cursor area in sel_focus's coordinate space if size != ZERO
    ime_cursor_area: Rect,
    last_ime_rect: Rect,
    has_reported_ime_not_supported: bool,
    input: Input,
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
    // Optional new target for selection focus. bool is true if this also gains key focus.
    pending_sel_focus: Option<PendingSelFocus>,
    pending_nav_focus: PendingNavFocus,
    action_moved: Option<ActionMoved>,
    pub(crate) action_redraw: Option<ActionRedraw>,
    action_close: Option<ActionClose>,
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
            ime_is_enabled: false,
            old_ime_target: None,
            ime_cursor_area: Rect::ZERO,
            last_ime_rect: Rect::ZERO,
            has_reported_ime_not_supported: false,
            input: Input::default(),
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
            pending_sel_focus: None,
            pending_nav_focus: PendingNavFocus::None,
            action_moved: None,
            action_redraw: None,
            action_close: None,
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
    #[must_use]
    pub(crate) fn full_configure(
        &mut self,
        sizer: &dyn ThemeSize,
        node: Node,
    ) -> Option<ActionResize> {
        let id = Id::ROOT.make_child(self.window_id.get().cast());

        log::debug!(target: "kas_core::event", "full_configure of Window{id}");

        // These are recreated during configure:
        self.nav_fallback = None;

        let mut cx = ConfigCx::new(sizer, self);
        cx.configure(node, id);
        let resize = cx.resize;
        // Ignore cx.redraw: we can assume a redraw will happen
        self.action_moved = Some(ActionMoved);
        resize
    }

    /// Construct a [`EventCx`] referring to this state
    ///
    /// Invokes the given closure on this [`EventCx`].
    #[inline]
    #[must_use]
    pub(crate) fn with<'a, F: FnOnce(&mut EventCx)>(
        &'a mut self,
        runner: &'a mut dyn RunnerT,
        theme: &'a dyn ThemeSize,
        window: &'a dyn WindowDataErased,
        f: F,
    ) -> Option<ActionResize> {
        let mut cx = EventCx {
            runner,
            window,
            cx: ConfigCx::new(theme, self),
            target_is_disabled: false,
            last_child: None,
            scroll: Scroll::None,
            resize_window: None,
        };
        f(&mut cx);
        let resize = cx.resize.or(cx.resize_window);
        let redraw = cx.redraw;
        self.action_redraw = self.action_redraw.or(redraw);
        resize
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

    /// Notify that a widget must be redrawn
    ///
    /// This method is designed to support partial redraws though these are not
    /// currently implemented. See also [`Self::action_redraw`].
    #[inline]
    pub fn redraw(&mut self, id: impl HasId) {
        // NOTE: redraws are fast enough not to bother handling locally
        let _ = id;

        self.action_redraw = Some(ActionRedraw);
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
        self.action_moved = Some(ActionMoved);
    }

    /// Request that the window be closed
    #[inline]
    pub fn close_own_window(&mut self) {
        self.action_close = Some(ActionClose);
    }

    /// Notify of an [`ActionMoved`] or `Option<ActionMoved>`
    #[inline]
    pub fn action_moved(&mut self, action: impl Into<Option<ActionMoved>>) {
        self.action_moved = self.action_moved.or(action.into());
    }

    /// Notify of an [`ActionRedraw`] or `Option<ActionRedraw>`
    ///
    /// This will force a redraw of the whole window. See also [`Self::redraw`].
    #[inline]
    pub fn action_redraw(&mut self, action: impl Into<Option<ActionRedraw>>) {
        self.action_redraw = self.action_redraw.or(action.into());
    }
}

/// Widget configuration and update context
///
/// This type supports access to [`EventState`] via [`Deref`] / [`DerefMut`]
/// and to [`SizeCx`] via [`Self::size_cx`].
#[must_use]
pub struct ConfigCx<'a> {
    theme: &'a dyn ThemeSize,
    state: &'a mut EventState,
    resize: Option<ActionResize>,
    redraw: Option<ActionRedraw>,
}

impl<'a> Deref for ConfigCx<'a> {
    type Target = EventState;
    fn deref(&self) -> &EventState {
        self.state
    }
}
impl<'a> DerefMut for ConfigCx<'a> {
    fn deref_mut(&mut self) -> &mut EventState {
        self.state
    }
}

impl<'a> ConfigCx<'a> {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub fn new(sh: &'a dyn ThemeSize, ev: &'a mut EventState) -> Self {
        ConfigCx {
            theme: sh,
            state: ev,
            resize: None,
            redraw: None,
        }
    }

    /// Access a [`SizeCx`]
    #[inline]
    pub fn size_cx<'b>(&'b mut self) -> SizeCx<'b>
    where
        'a: 'b,
    {
        SizeCx::new(self.state, self.theme)
    }

    /// Configure a widget
    ///
    /// All widgets must be configured after construction; see
    /// [widget lifecycle](crate::Widget#widget-lifecycle) and
    /// [configuration](Events#configuration).
    /// Widgets must always be sized after configuration.
    ///
    /// Assigns `id` to the widget. This must be valid and is usually
    /// constructed with [`Events::make_child_id`].
    #[inline]
    pub fn configure(&mut self, mut widget: Node<'_>, id: Id) {
        // This recurses; avoid passing existing state in
        // (Except redraw: this doesn't matter.)
        let start_resize = std::mem::take(&mut self.resize);
        widget._configure(self, id);
        self.resize = self.resize.or(start_resize);
    }

    /// Update a widget
    ///
    /// All widgets must be updated after input data changes; see
    /// [update](Events#update).
    #[inline]
    pub fn update(&mut self, mut widget: Node<'_>) {
        // This recurses; avoid passing existing state in
        // (Except redraw: this doesn't matter.)
        let start_resize = std::mem::take(&mut self.resize);
        widget._update(self);
        self.resize = self.resize.or(start_resize);
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
                    self.redraw();
                    self.disabled.remove(i);
                }
                return;
            }
        }
        if disable {
            self.redraw();
            self.disabled.push(target);
        }
    }

    /// Set a target for messages of a specific type when sent to `Id::default()`
    ///
    /// Messages of this type sent to `Id::default()` from any window will be
    /// sent to `id`.
    pub fn set_send_target_for<M: Debug + 'static>(&mut self, id: Id) {
        let type_id = TypeId::of::<M>();
        self.pending_send_targets.push((type_id, id));
    }

    /// Notify that a widget must be redrawn
    ///
    /// "The current widget" is inferred from the widget tree traversal through
    /// which the `EventCx` is made accessible. The resize is handled locally
    /// during the traversal unwind if possible.
    #[inline]
    pub fn redraw(&mut self) {
        self.redraw = Some(ActionRedraw);
    }

    /// Require that the current widget (and its descendants) be resized
    ///
    /// "The current widget" is inferred from the widget tree traversal through
    /// which the `EventCx` is made accessible. The resize is handled locally
    /// during the traversal unwind if possible.
    #[inline]
    pub fn resize(&mut self) {
        self.resize = Some(ActionResize);
    }

    #[inline]
    pub(crate) fn needs_redraw(&self) -> bool {
        self.redraw.is_some()
    }

    #[inline]
    pub(crate) fn needs_resize(&self) -> bool {
        self.resize.is_some()
    }

    #[inline]
    pub(crate) fn set_resize(&mut self, resize: Option<ActionResize>) {
        self.resize = resize;
    }
}

/// Event handling context
///
/// This type supports access to [`ConfigCx`] and [`EventState`] via (recursive)
/// [`Deref`] / [`DerefMut`] and to [`SizeCx`] via [`ConfigCx::size_cx`].
#[must_use]
pub struct EventCx<'a> {
    runner: &'a mut dyn RunnerT,
    window: &'a dyn WindowDataErased,
    cx: ConfigCx<'a>,
    pub(crate) target_is_disabled: bool,
    last_child: Option<usize>,
    scroll: Scroll,
    resize_window: Option<ActionResize>,
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

    /// Check for clean flags pre-recursion
    fn pre_recursion(&self) {
        debug_assert!(self.resize.is_none());
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
    }

    /// Clean up post-recursion
    fn post_recursion(&mut self) {
        self.last_child = None;
        self.scroll = Scroll::None;
        self.resize_window = self.resize_window.or(self.resize);
        self.resize = None;
    }
}
