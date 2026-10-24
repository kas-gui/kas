// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context: key handling and selection focus

use super::*;
#[allow(unused)] use crate::Events;
use crate::HasId;
use crate::event::{Command, Event, FocusSource};
use crate::geom::{Rect, Size};
use crate::util::WidgetHierarchy;
use crate::{Id, Node, Tile, TileExt};
use winit::event::{ElementState, Ime, KeyEvent};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};
use winit::window::{
    ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData, ImeRequestError,
};

#[derive(Debug)]
struct LostFocus {
    id: Id,
    key: bool,
    ime: bool,
}

/// Keyboard and IME focus tracking
#[derive(Debug, Default)]
pub(super) struct Input {
    focus: Option<Id>,
    key_focus: bool,
    ime_focus: bool,
    lost_focus: Option<LostFocus>,
    new_sel_focus: Option<FocusSource>,
    new_key_focus: bool,
    new_ime_focus: Option<(ImeHint, ImePurpose)>,
    ime_surrounding_text: Option<ImeSurroundingText>,
    old_ime_target: Option<Id>,
    /// Rect is cursor area in sel_focus's coordinate space if size != ZERO
    ime_cursor_area: Rect,
    last_ime_rect: Rect,
    has_reported_ime_not_supported: bool,
}

impl Input {
    #[inline]
    pub(super) fn sel_focus(&self) -> Option<&Id> {
        self.focus.as_ref()
    }

    #[inline]
    pub(super) fn key_focus(&self) -> Option<&Id> {
        self.key_focus.then_some(self.focus.as_ref()).flatten()
    }

    #[inline]
    pub(super) fn ime_focus(&self) -> Option<&Id> {
        self.ime_focus.then_some(self.focus.as_ref()).flatten()
    }

    /// Clear sel, key and ime focus on target
    ///
    /// Returns `true` on success.
    pub(super) fn clear_sel_socus_on(&mut self, target: &Id) -> bool {
        if *target != self.sel_focus() {
            return true; // nothing to do
        } else if let Some(ref focus) = self.lost_focus
            && focus.id != target
        {
            return false; // conflicting focus request
        }

        let old_focus = self.focus.take().unwrap();
        let key = std::mem::take(&mut self.key_focus);
        let ime = std::mem::take(&mut self.ime_focus);

        if let Some(ref mut focus) = self.lost_focus {
            // Upgrade partial-loss to full-loss
            focus.key |= key;
            focus.ime |= ime;
        } else {
            self.lost_focus = Some(LostFocus {
                id: old_focus,
                key,
                ime,
            });
        }

        true
    }

    #[inline]
    pub(super) fn has_pending_changes(&self) -> bool {
        self.lost_focus.is_some() || self.new_sel_focus.is_some() || self.new_key_focus
    }

    pub(super) fn frame_update(&mut self, window: &dyn WindowDataErased, widget: &dyn Tile) {
        let Some(target) = self.ime_focus() else {
            return;
        };
        let mut rect = self.last_ime_rect;

        // Set IME cursor area, if moved.
        if let Some((widget_rect, translation)) = widget.find_tile_rect(target) {
            if self.ime_cursor_area.size != Size::ZERO {
                rect = self.ime_cursor_area;
            } else {
                rect = widget_rect;
            }
            rect += translation;
        }

        if rect != self.last_ime_rect || self.ime_surrounding_text.is_some() {
            let mut data = ImeRequestData::default();
            data.cursor_area = Some((
                rect.pos.as_physical().into(),
                rect.size.as_physical().into(),
            ));
            data.surrounding_text = self.ime_surrounding_text.take();

            let req = winit::window::ImeRequest::Update(data);
            match window.ime_request(req) {
                Ok(()) => (),
                Err(e) => log::warn!("Unexpected IME error: {e}"),
            }
            self.last_ime_rect = rect;
        }
    }
}

impl EventState {
    pub(crate) fn clear_access_key_bindings(&mut self) {
        self.access_keys.clear();
    }

    pub(crate) fn add_access_key_binding(&mut self, id: &Id, key: &Key) -> bool {
        if !self.modifiers.alt_key() && !self.config.alt_bypass {
            return false;
        }

        if self.access_keys.contains_key(key) {
            false
        } else {
            self.access_keys.insert(key.clone(), id.clone());
            self.modifiers.alt_key()
        }
    }

    /// Get whether this widget has input focus
    ///
    /// Return values:
    ///
    /// -   `None` if the widget does not have selection focus
    /// -   `Some(false)` if the widget has selection focus but not keyboard or
    ///     IME focus
    /// -   `Some(true)` if the widget has keyboard and/or IME focus
    #[inline]
    pub fn has_input_focus(&self, w_id: &Id) -> Option<bool> {
        if *w_id == self.input.focus {
            Some(self.input.key_focus || self.input.ime_focus)
        } else {
            None
        }
    }

    /// Get the current modifier state
    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    /// Set IME cursor area
    ///
    /// This should be called after receiving
    /// <code>[Event::Ime][]([Ime::Enabled][crate::event::Ime::Enabled])</code>,
    /// and any time that the `rect` parameter changes, until
    /// <code>[Event::Ime][]([Ime::Disabled][crate::event::Ime::Disabled])</code>
    /// is received.
    ///
    /// This sets the text cursor's area, `rect`, relative to the widget's own
    /// coordinate space. If never called, then the widget's whole rect is used.
    ///
    /// This does nothing if `target` does not have IME-enabled input focus.
    #[inline]
    pub fn set_ime_cursor_area(&mut self, target: &Id, rect: Rect) {
        if *target == self.input.ime_focus() {
            self.input.ime_cursor_area = rect;
        }
    }

    /// Explicitly clear Input Method Editor focus on `target`
    ///
    /// This method may be used to disable IME focus while retaining selection
    /// focus. IME focus is lost automatically when selection focus is lost.
    #[inline]
    pub fn cancel_ime_focus(&mut self, target: &Id) {
        if *target != self.input.ime_focus() {
            return;
        }
        self.input.ime_focus = false;

        if let Some(ref mut focus) = self.input.lost_focus {
            if focus.id == target {
                focus.ime = true;
            } else {
                // Loss of focus on another target implies sel focus changed
                // recently without sending Event::SelFocus yet. Do nothing.
            }
        } else {
            self.input.lost_focus = Some(LostFocus {
                id: target.clone(),
                key: false,
                ime: true,
            });
        }
    }
}

impl<'a> EventCx<'a> {
    /// Request Input Method Editor (IME) focus
    ///
    /// This method requests [selection focus](Self::request_sel_focus),
    /// [keyboard input focus](Self::request_key_focus) (required since some
    /// Input Method Editors send synthesized key events) and IME focus.
    ///
    /// Does nothing if this focus has already been obtained.
    /// To change the `hint` or `purpose` call [`EventState::cancel_ime_focus`] first.
    ///
    /// Once enabled, `target` should receive [`Event::Ime`] events.
    pub fn request_ime_focus(
        &mut self,
        target: Id,
        source: FocusSource,
        hint: ImeHint,
        purpose: ImePurpose,
        surrounding_text: impl Fn() -> Option<ImeSurroundingText>,
    ) {
        if self.input.new_ime_focus.is_some() {
            return; // conflicting request
        }

        if !self.request_sel_focus(target.clone(), source) {
            return;
        }

        self.request_key_focus(target, source);

        if !self.input.ime_focus {
            self.input.new_ime_focus = Some((hint, purpose));
            self.input.ime_surrounding_text = surrounding_text();
        }
    }

    /// Request keyboard (input) focus
    ///
    /// This method requests [selection focus](Self::request_sel_focus) and
    /// keyboard input focus. On success, the `target` widget will receive
    /// [`Event::KeyFocus`] then [`Event::Key`] for each key press and release.
    ///
    /// Note: automatic translation of key events to [`Event::Command`] is
    /// disabled while key focus is active.
    ///
    /// Since key focus requires [selection focus](Self::request_sel_focus),
    /// this method requests that too.
    ///
    /// The `source` parameter is used by [`Event::SelFocus`].
    #[inline]
    pub fn request_key_focus(&mut self, target: Id, source: FocusSource) {
        if !self.request_sel_focus(target, source) {
            return;
        }

        if !self.input.key_focus {
            self.input.key_focus = true;
            self.input.new_key_focus = true;
        }
    }

    /// Test whether `target` has selection focus
    #[inline]
    pub fn has_sel_focus(&self, target: &Id) -> bool {
        self.input.focus.as_ref() == Some(target)
    }

    /// Request selection focus
    ///
    /// To prevent multiple simultaneous selections (e.g. of text) in the UI,
    /// only widgets with "selection focus" are allowed to select things.
    /// Selection focus is implied by character focus. [`Event::LostSelFocus`]
    /// is sent when selection focus is lost; in this case any existing
    /// selection should be cleared.
    ///
    /// Selection focus is a pre-requisite for
    /// [key focus](Self::request_key_focus) and
    /// [IME focus](Self::request_ime_focus).
    /// Selection focus implies navigation focus.
    ///
    /// Returns `true` if and only if the request succeeds. On success, the
    /// `target` will receive event [`Event::SelFocus`] with the input `source`
    /// parameter.
    #[inline]
    pub fn request_sel_focus(&mut self, target: Id, source: FocusSource) -> bool {
        if target == self.input.sel_focus() {
            return true;
        } else if let Some(id) = self.input.sel_focus().cloned() {
            if !self.input.clear_sel_socus_on(&id) {
                return false;
            }
        }

        self.set_nav_focus(target.clone(), source);

        self.input.focus = Some(target);
        self.input.new_sel_focus = Some(source);
        true
    }

    /// Visually depress a widget via a key code
    ///
    /// When a button-like widget is activated by a key it may call this to
    /// ensure the widget is visually depressed until the key is released.
    /// The widget will not receive a notification of key-release but will be
    /// redrawn automatically.
    ///
    /// Note that keyboard shortcuts and mnemonics should usually match against
    /// the "logical key". [`PhysicalKey`] is used here since the the logical key
    /// may be changed by modifier keys.
    ///
    /// Does nothing when `code` is `None`.
    pub fn depress_with_key(&mut self, id: impl HasId, code: impl Into<Option<PhysicalKey>>) {
        fn inner(state: &mut EventState, id: Id, code: PhysicalKey) {
            if state
                .key_depress
                .get(&code)
                .map(|target| *target == id)
                .unwrap_or(false)
            {
                return;
            }

            state.key_depress.insert(code, id.clone());
            state.redraw(id);
        }

        if let Some(code) = code.into() {
            inner(self, id.has_id(), code);
        }
    }

    /// Update surrounding text used by Input Method Editor
    pub fn update_ime_surrounding_text(
        &mut self,
        target: &Id,
        surrounding_text: ImeSurroundingText,
    ) {
        if *target != self.input.ime_focus() {
            return;
        }

        self.input.ime_surrounding_text = Some(surrounding_text);
    }

    pub(super) fn clear_ime_focus(&mut self, id: Id) {
        // NOTE: we assume that winit will send us Ime::Disabled. Until then,
        // other IME events (e.g. a final Commit) are routed to `old_ime_target`.
        match self.window.ime_request(ImeRequest::Disable) {
            Ok(()) => self.input.old_ime_target = Some(id),
            Err(e) => log::error!("Unexpected IME error: {e}"),
        }
        self.input.ime_focus = false;
        self.input.ime_cursor_area = Rect::ZERO;
    }

    pub(super) fn keyboard_input(
        &mut self,
        mut widget: Node<'_>,
        mut event: KeyEvent,
        is_synthetic: bool,
    ) {
        if let Some(id) = self.input.key_focus().cloned() {
            // TODO(winit): https://github.com/rust-windowing/winit/issues/3038
            let mut mods = self.modifiers;
            mods.remove(ModifiersState::SHIFT);
            if !mods.is_empty()
                || event
                    .text
                    .as_ref()
                    .and_then(|t| t.chars().next())
                    .map(|c| c.is_control())
                    .unwrap_or(false)
            {
                event.text = None;
            }

            if self.send_event(widget.re(), id, Event::Key(&event, is_synthetic)) {
                return;
            }
        }

        if event.state == ElementState::Pressed && !is_synthetic {
            self.start_key_event(widget, &event);
        } else if event.state == ElementState::Released
            && self.key_depress.remove(&event.physical_key).is_some()
        {
            self.redraw();
        }
    }

    pub(super) fn start_key_event(&mut self, mut widget: Node<'_>, event: &KeyEvent) {
        let id = widget.id();
        log::trace!(
            "start_key_event: window={id}, physical_key={:?}, logical_key={:?}, without_modifiers={:?}",
            event.physical_key,
            event.logical_key,
            event.key_without_modifiers
        );

        let opt_cmd = self
            .config
            .shortcuts()
            .try_match_event(self.modifiers, event);

        if Some(Command::Exit) == opt_cmd {
            self.runner.exit();
            return;
        } else if Some(Command::Close) == opt_cmd {
            self.handle_close();
            return;
        } else if let Some(cmd) = opt_cmd {
            let mut targets = vec![];
            let mut send = |_self: &mut Self, id: Id, cmd| -> bool {
                if !targets.contains(&id) {
                    let event = Event::Command(cmd, Some(event.physical_key));
                    let used = _self.send_event(widget.re(), id.clone(), event);
                    targets.push(id);
                    used
                } else {
                    false
                }
            };

            if (self.input.key_focus || cmd.suitable_for_sel_focus())
                && let Some(ref id) = self.input.focus
                && send(self, id.clone(), cmd)
            {
                return;
            }

            if !self.modifiers.alt_key()
                && let Some(id) = self.nav_focus().cloned()
                && send(self, id, cmd)
            {
                return;
            }

            if let Some(id) = self
                .popups
                .last()
                .filter(|popup| popup.is_sized)
                .map(|popup| popup.desc.id.clone())
                && send(self, id, cmd)
            {
                return;
            }

            let fallback = self.nav.fallback.clone().unwrap_or(id);
            if send(self, fallback, cmd) {
                return;
            }

            if matches!(cmd, Command::Debug) {
                let over_id = self.mouse.over_id();
                let hier = WidgetHierarchy::new(widget.as_tile(), over_id.clone());
                log::debug!("Widget hierarchy (filter={over_id:?}): {hier}");
                return;
            }
        }

        // Next priority goes to access keys when Alt is held or alt_bypass is true
        let target = self.access_keys.get(&event.logical_key).cloned();

        if let Some(id) = target {
            self.close_non_ancestors_of(Some(&id));

            if let Some(id) = self.nav_next(widget.as_tile(), Some(&id), NavAdvance::None) {
                self.request_nav_focus(id, FocusSource::Key);
            }

            let event = Event::Command(Command::Activate, Some(event.physical_key));
            self.send_event(widget, id, event);
        } else if self.config.nav_focus && opt_cmd == Some(Command::Tab) {
            let shift = self.modifiers.shift_key();
            self.next_nav_focus(None, shift, FocusSource::Key);
        }
    }

    pub(super) fn modifiers_changed(&mut self, state: ModifiersState) {
        if state.alt_key() != self.modifiers.alt_key() {
            // This controls drawing of access key indicators
            self.redraw();
        }
        self.modifiers = state;
    }

    pub(super) fn ime_event(&mut self, mut widget: Node<'_>, ime: Ime) {
        if let Some(target) = self.input.old_ime_target.clone() {
            match ime {
                Ime::Disabled => {
                    // The session we disabled has ended
                    self.input.old_ime_target = None;
                    self.send_event(widget, target, Event::Ime(super::Ime::Disabled));
                    return;
                }
                Ime::Enabled => {
                    // A new session has begun without Disabled being received
                    // for the old one: synthesise it so that none is left dangling.
                    self.input.old_ime_target = None;
                    self.send_event(widget.re(), target, Event::Ime(super::Ime::Disabled));
                }
                Ime::Preedit(ref text, cursor) => {
                    let event = super::Ime::Preedit { text, cursor };
                    self.send_event(widget, target, Event::Ime(event));
                    return;
                }
                Ime::Commit(ref text) => {
                    // Late commit (e.g. of a pending pre-edit on focus loss)
                    let event = super::Ime::Commit { text };
                    self.send_event(widget, target, Event::Ime(event));
                    return;
                }
                Ime::DeleteSurrounding {
                    before_bytes,
                    after_bytes,
                } => {
                    let event = super::Ime::DeleteSurrounding {
                        before_bytes,
                        after_bytes,
                    };
                    self.send_event(widget, target, Event::Ime(event));
                    return;
                }
            }
        }

        let Some(id) = self.input.ime_focus().cloned() else {
            log::debug!("ime_event: no target; event dropped");
            return;
        };

        let event = match ime {
            Ime::Enabled => super::Ime::Enabled,
            Ime::Preedit(ref text, cursor) => super::Ime::Preedit { text, cursor },
            Ime::Commit(ref text) => super::Ime::Commit { text },
            Ime::DeleteSurrounding {
                before_bytes,
                after_bytes,
            } => super::Ime::DeleteSurrounding {
                before_bytes,
                after_bytes,
            },
            Ime::Disabled => {
                // IME disabled by external cause
                self.input.ime_focus = false;
                self.input.ime_cursor_area = Rect::ZERO;

                super::Ime::Disabled
            }
        };

        self.send_event(widget, id, Event::Ime(event));
    }

    pub(super) fn flush_pending_input_focus(&mut self, mut widget: Node<'_>) {
        if let Some(focus) = self.input.lost_focus.take() {
            if focus.ime {
                self.clear_ime_focus(focus.id.clone());
            }

            if focus.key {
                self.send_event(widget.re(), focus.id.clone(), Event::LostKeyFocus);
            }

            if focus.id != self.input.sel_focus() {
                self.send_event(widget.re(), focus.id, Event::LostSelFocus);
            }
        }

        if let Some(source) = self.input.new_sel_focus.take() {
            if let Some(id) = self.input.focus.clone() {
                self.send_event(widget.re(), id.clone(), Event::SelFocus(source));
            } else {
                debug_assert!(false);
            }
        }
        if std::mem::take(&mut self.input.new_key_focus) {
            if let Some(id) = self.input.focus.clone() {
                self.send_event(widget.re(), id.clone(), Event::KeyFocus);
            } else {
                debug_assert!(false);
            }
        }

        if let Some(hint_and_purpose) = self.input.new_ime_focus.take() {
            let Some(id) = self.input.focus.as_ref() else {
                debug_assert!(false);
                return;
            };

            let mut capabilities = ImeCapabilities::new().with_hint_and_purpose();
            if self.input.ime_surrounding_text.is_some() {
                capabilities = capabilities.with_surrounding_text();
            }

            let mut rect = Rect::ZERO;
            if let Some((widget_rect, translation)) = widget.as_tile().find_tile_rect(id) {
                capabilities = capabilities.with_cursor_area();

                if self.input.ime_cursor_area.size != Size::ZERO {
                    rect = self.input.ime_cursor_area;
                } else {
                    rect = widget_rect;
                };
                rect += translation;
            }
            self.input.last_ime_rect = rect;

            let mut data = ImeRequestData::default();
            data.hint_and_purpose = Some(hint_and_purpose);
            data.cursor_area = Some((
                rect.pos.as_physical().into(),
                rect.size.as_physical().into(),
            ));
            data.surrounding_text = self.input.ime_surrounding_text.take();

            let req = ImeEnableRequest::new(capabilities, data).unwrap();
            match self.window.ime_request(ImeRequest::Enable(req)) {
                Ok(()) => {
                    // NOTE: we could store (a clone of) data here, but we don't
                    // need to: we only need to pass changed properties on update.
                    self.input.ime_focus = true;
                }
                Err(ImeRequestError::NotSupported) => {
                    if !self.input.has_reported_ime_not_supported {
                        log::warn!("Failed to start Input Method Editor: not supported");
                        self.input.has_reported_ime_not_supported = true;
                    }
                }
                Err(e) => log::error!("Unexpected IME error: {e}"),
            }
        }
    }
}
