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
use winit::dpi::{LogicalPosition, LogicalSize};
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
    pub(super) fn clear_sel_socus_on(&mut self, target: &Id) {
        if *target != self.sel_focus() {
            return; // nothing to do
        }
        let old_focus = self.focus.take().unwrap();
        let key = std::mem::take(&mut self.key_focus);
        let ime = std::mem::take(&mut self.ime_focus);

        if let Some(ref mut focus) = self.lost_focus {
            if focus.id == target {
                // Upgrade partial-loss to full-loss
                focus.key |= key;
                focus.ime |= ime;
            } else {
                // Loss of focus on another target implies old_focus changed
                // recently without sending Event::SelFocus yet. Do nothing.
            }
        } else {
            self.lost_focus = Some(LostFocus {
                id: old_focus,
                key,
                ime,
            });
        }
    }

    #[inline]
    pub(super) fn has_pending_changes(&self) -> bool {
        self.lost_focus.is_some() || self.new_sel_focus.is_some() || self.new_key_focus
    }

    pub(super) fn frame_update(&mut self, window: &dyn WindowDataErased, widget: &dyn Tile) {
        // Set IME cursor area, if moved.
        if let Some(target) = self.ime_focus()
            && let Some((mut rect, translation)) = widget.find_tile_rect(target)
        {
            if self.ime_cursor_area.size != Size::ZERO {
                rect = self.ime_cursor_area;
            }
            rect += translation;
            if rect != self.last_ime_rect {
                let data = ImeRequestData::default().with_cursor_area(
                    rect.pos.as_physical().into(),
                    rect.size.as_physical().into(),
                );
                let req = winit::window::ImeRequest::Update(data);
                match window.ime_request(req) {
                    Ok(()) => (),
                    Err(e) => log::warn!("Unexpected IME error: {e}"),
                }
                self.last_ime_rect = rect;
            }
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
    /// Request keyboard input focus
    ///
    /// When granted, the widget will receive [`Event::KeyFocus`] followed by
    /// [`Event::Key`] for each key press / release. Note that this disables
    /// translation of key events to [`Event::Command`] while key focus is
    /// active.
    ///
    /// The `source` parameter is used by [`Event::SelFocus`].
    ///
    /// Key focus implies sel focus (see [`Self::request_sel_focus`]) and
    /// navigation focus. It also clears IME focus.
    #[inline]
    pub fn request_key_focus(&mut self, target: Id, source: FocusSource) {
        self.request_sel_focus(target, source);
        if !self.input.key_focus {
            self.input.key_focus = true;
            self.input.new_key_focus = true;
        }
    }

    /// Request selection focus
    ///
    /// To prevent multiple simultaneous selections (e.g. of text) in the UI,
    /// only widgets with "selection focus" are allowed to select things.
    /// Selection focus is implied by character focus. [`Event::LostSelFocus`]
    /// is sent when selection focus is lost; in this case any existing
    /// selection should be cleared.
    ///
    /// The `source` parameter is used by [`Event::SelFocus`].
    ///
    /// Selection focus implies navigation focus.
    ///
    /// When key focus is lost, [`Event::LostSelFocus`] is sent.
    #[inline]
    pub fn request_sel_focus(&mut self, target: Id, source: FocusSource) {
        if target == self.input.sel_focus() {
            return;
        } else if let Some(id) = self.input.sel_focus().cloned() {
            self.input.clear_sel_socus_on(&id);
        }

        self.set_nav_focus(target.clone(), source);

        self.input.focus = Some(target);
        self.input.new_sel_focus = Some(source);
    }

    /// Request Input Method Editor (IME) focus
    ///
    /// IME focus requires selection focus (see
    /// [`EventState::request_key_focus`], [`EventState::request_sel_focus`]).
    /// The request fails if this has not already been obtained (wait for
    /// [`Event::SelFocus`] or [`Event::KeyFocus`] before calling this method).
    ///
    /// This method enables IME focus, replacing the existing focus. This should
    /// be used both to initially enable IME input and to replace the `hint`,
    /// `purpose` or `surrounding_text`.
    ///
    /// Once enabled, `target` should receive [`Event::Ime`] events.
    pub fn replace_ime_focus(
        &mut self,
        target: Id,
        hint: ImeHint,
        purpose: ImePurpose,
        mut surrounding_text: Option<ImeSurroundingText>,
    ) {
        if target != self.input.focus {
            return;
        }

        if let Some(id) = self.input.ime_focus().cloned() {
            self.clear_ime_focus(id);
        }

        let mut capabilities = ImeCapabilities::new()
            .with_hint_and_purpose()
            .with_cursor_area();
        if surrounding_text.is_some() {
            capabilities = capabilities.with_surrounding_text();
        }

        // NOTE: we provide bogus cursor area and update in `frame_update`;
        // the API does not allow to only provide this later.
        let position = LogicalPosition::new(0, 0);
        let size = LogicalSize::new(0, 0);

        let mut data = ImeRequestData::default()
            .with_hint_and_purpose(hint, purpose)
            .with_cursor_area(position.into(), size.into());
        if let Some(surrounding) = surrounding_text.take() {
            data = data.with_surrounding_text(surrounding);
        }

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
    pub fn update_ime_surrounding_text(&self, target: &Id, surrounding_text: ImeSurroundingText) {
        if *target != self.input.ime_focus() {
            return;
        }

        let data = ImeRequestData::default().with_surrounding_text(surrounding_text);
        let req = ImeRequest::Update(data);
        match self.window.ime_request(req) {
            Ok(()) => (),
            Err(e) => log::error!("Unexpected IME error: {e}"),
        }
    }

    pub(super) fn clear_ime_focus(&mut self, id: Id) {
        // NOTE: we assume that winit will send us Ime::Disabled
        self.input.old_ime_target = Some(id);
        self.window.ime_request(ImeRequest::Disable).unwrap();
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
            self.start_key_event(widget, event.key_without_modifiers, event.physical_key);
        } else if event.state == ElementState::Released
            && self.key_depress.remove(&event.physical_key).is_some()
        {
            self.redraw();
        }
    }

    pub(super) fn start_key_event(&mut self, mut widget: Node<'_>, vkey: Key, code: PhysicalKey) {
        let id = widget.id();
        log::trace!("start_key_event: window={id}, vkey={vkey:?}, physical_key={code:?}");

        let opt_cmd = self.config.shortcuts().try_match(self.modifiers, &vkey);

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
                    let event = Event::Command(cmd, Some(code));
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
                log::debug!("Widget heirarchy (filter={over_id:?}): {hier}");
                return;
            }
        }

        // Next priority goes to access keys when Alt is held or alt_bypass is true
        let target = self.access_keys.get(&vkey).cloned();

        if let Some(id) = target {
            self.close_non_ancestors_of(Some(&id));

            if let Some(id) = self.nav_next(widget.as_tile(), Some(&id), NavAdvance::None) {
                self.request_nav_focus(id, FocusSource::Key);
            }

            let event = Event::Command(Command::Activate, Some(code));
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

    pub(super) fn ime_event(&mut self, widget: Node<'_>, ime: Ime) {
        if ime == Ime::Disabled
            && let Some(target) = self.input.old_ime_target.take()
        {
            // Assume that we disabled IME when old_ime_target is set
            self.send_event(widget, target, Event::Ime(super::Ime::Disabled));
            return;
        }

        if let Some(id) = self.input.ime_focus().cloned() {
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

        if let Some(source) = self.input.new_sel_focus.take()
            && let Some(id) = self.input.sel_focus().cloned()
        {
            self.send_event(widget.re(), id, Event::SelFocus(source));
        }
        if self.input.new_key_focus
            && let Some(id) = self.input.key_focus().cloned()
        {
            self.input.new_key_focus = false;
            self.send_event(widget.re(), id, Event::KeyFocus);
        }
    }
}
