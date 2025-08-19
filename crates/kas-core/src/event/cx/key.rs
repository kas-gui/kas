// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context: key handling and selection focus

use super::{EventCx, EventState, NavAdvance};
#[allow(unused)] use crate::Events;
use crate::event::{Command, Event, FocusSource};
use crate::util::WidgetHierarchy;
use crate::{Action, HasId};
use crate::{Id, Node, geom::Rect, runner::WindowDataErased};
use winit::event::{ElementState, Ime, KeyEvent};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};
use winit::window::ImePurpose;

#[derive(Debug)]
pub(super) struct PendingSelFocus {
    target: Option<Id>,
    key_focus: bool,
    ime: Option<ImePurpose>,
    source: FocusSource,
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

    /// Get the current modifier state
    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    /// Get whether this widget has `(key_focus, sel_focus)`
    ///
    /// -   `key_focus`: implies this widget receives keyboard input
    /// -   `sel_focus`: implies this widget is allowed to select things
    ///
    /// Note that `key_focus` implies `sel_focus`.
    #[inline]
    pub fn has_key_focus(&self, w_id: &Id) -> (bool, bool) {
        let sel_focus = *w_id == self.sel_focus;
        (sel_focus && self.key_focus, sel_focus)
    }

    #[inline]
    pub(super) fn key_focus(&self) -> Option<Id> {
        if self.key_focus { self.sel_focus.clone() } else { None }
    }

    pub(super) fn clear_key_focus(&mut self) {
        if self.key_focus {
            if let Some(ref mut pending) = self.pending_sel_focus {
                if pending.target == self.sel_focus {
                    pending.key_focus = false;
                }
            } else {
                self.pending_sel_focus = Some(PendingSelFocus {
                    target: None,
                    key_focus: false,
                    ime: None,
                    source: FocusSource::Synthetic,
                });
            }
        }
    }

    pub(super) fn clear_sel_socus_on(&mut self, target: &Id) {
        if let Some(id) = self.sel_focus.as_ref()
            && target.is_ancestor_of(id)
        {
            if let Some(pending) = self.pending_sel_focus.as_mut() {
                if pending.target.as_ref() == Some(id) {
                    pending.target = None;
                    pending.key_focus = false;
                } else {
                    // We have a new focus target, hence the old one will be cleared
                }
            } else {
                self.pending_sel_focus = Some(PendingSelFocus {
                    target: None,
                    key_focus: false,
                    ime: None,
                    source: FocusSource::Synthetic,
                });
            }
        }
    }

    /// Request keyboard input focus
    ///
    /// When granted, the widget will receive [`Event::KeyFocus`] followed by
    /// [`Event::Key`] for each key press / release. Note that this disables
    /// translation of key events to [`Event::Command`] while key focus is
    /// active.
    ///
    /// Providing an [`ImePurpose`] enables Input Method Editor events
    /// (see [`Event::ImeFocus`]). TODO: this feature is incomplete; winit does
    /// not currently support setting surrounding text.
    ///
    /// The `source` parameter is used by [`Event::SelFocus`].
    ///
    /// Key focus implies sel focus (see [`Self::request_sel_focus`]) and
    /// navigation focus.
    #[inline]
    pub fn request_key_focus(&mut self, target: Id, ime: Option<ImePurpose>, source: FocusSource) {
        if self.nav_focus.as_ref() != Some(&target) {
            self.set_nav_focus(target.clone(), source);
        }

        self.pending_sel_focus = Some(PendingSelFocus {
            target: Some(target),
            key_focus: true,
            ime,
            source,
        });
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
        if self.nav_focus.as_ref() != Some(&target) {
            self.set_nav_focus(target.clone(), source);
        }

        if let Some(ref pending) = self.pending_sel_focus
            && pending.target.as_ref() == Some(&target)
        {
            return;
        }

        self.pending_sel_focus = Some(PendingSelFocus {
            target: Some(target),
            key_focus: false,
            ime: None,
            source,
        });
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

    /// End Input Method Editor focus on `target`, if present
    #[inline]
    pub fn cancel_ime_focus(&mut self, target: Id) {
        if let Some(pending) = self.pending_sel_focus.as_mut() {
            if pending.target.as_ref() == Some(&target) {
                pending.ime = None;
            }
        } else if self.ime.is_some() && self.sel_focus.as_ref() == Some(&target) {
            self.pending_sel_focus = Some(PendingSelFocus {
                target: Some(target),
                key_focus: self.key_focus,
                ime: None,
                source: FocusSource::Synthetic,
            });
        }
    }

    /// Set IME cursor area
    ///
    /// This should be called after receiving [`Event::ImeFocus`], and any time
    /// that the `rect` parameter changes, until [`Event::LostImeFocus`] is
    /// received.
    ///
    /// This sets the text cursor's area, `rect`, relative to the widget's own
    /// coordinate space. If never called, then the widget's whole rect is used.
    ///
    /// This does nothing if `target` does not have IME-enabled input focus.
    #[inline]
    pub fn set_ime_cursor_area(&mut self, target: &Id, rect: Rect) {
        if self.ime.is_some() && self.sel_focus.as_ref() == Some(target) {
            self.ime_cursor_area = rect;
        }
    }
}

impl<'a> EventCx<'a> {
    pub(super) fn keyboard_input(
        &mut self,
        mut widget: Node<'_>,
        mut event: KeyEvent,
        is_synthetic: bool,
    ) {
        if let Some(id) = self.key_focus() {
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
            self.start_key_event(widget, event.logical_key, event.physical_key);
        } else if event.state == ElementState::Released
            && let Some(id) = self.key_depress.remove(&event.physical_key)
        {
            self.redraw(id);
        }
    }

    pub(super) fn start_key_event(&mut self, mut widget: Node<'_>, vkey: Key, code: PhysicalKey) {
        log::trace!(
            "start_key_event: widget={}, vkey={vkey:?}, physical_key={code:?}",
            widget.id()
        );

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

            if (self.key_focus || cmd.suitable_for_sel_focus())
                && let Some(id) = self.sel_focus.clone()
                && send(self, id, cmd)
            {
                return;
            }

            if !self.modifiers.alt_key()
                && let Some(id) = self.nav_focus.clone()
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

            if let Some(id) = self.nav_fallback.clone()
                && send(self, id, cmd)
            {
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

            if let Some(id) = self.nav_next(widget.re(), Some(&id), NavAdvance::None) {
                self.request_nav_focus(id, FocusSource::Key);
            }

            let event = Event::Command(Command::Activate, Some(code));
            self.send_event(widget, id, event);
        } else if self.config.nav_focus && opt_cmd == Some(Command::Tab) {
            let shift = self.modifiers.shift_key();
            self.next_nav_focus(None, shift, FocusSource::Key);
        } else if opt_cmd == Some(Command::Escape)
            && let Some(id) = self.popups.last().map(|desc| desc.id)
        {
            self.close_window(id);
        }
    }

    pub(super) fn modifiers_changed(&mut self, state: ModifiersState) {
        if state.alt_key() != self.modifiers.alt_key() {
            // This controls drawing of access key indicators
            self.window_action(Action::REDRAW);
        }
        self.modifiers = state;
    }

    pub(super) fn ime_event(&mut self, widget: Node<'_>, ime: Ime) {
        match ime {
            winit::event::Ime::Enabled => {
                // We expect self.ime.is_some(), but it's possible that the request is outdated
                if self.ime.is_some()
                    && let Some(id) = self.sel_focus.clone()
                {
                    self.send_event(widget, id, Event::ImeFocus);
                }
            }
            winit::event::Ime::Disabled => {
                // We can only assume that this is received due to us disabling
                // IME if self.old_ime_target is set, and is otherwise due to an
                // external cause.
                let mut target = self.old_ime_target.take();
                if target.is_none() && self.ime.is_some() {
                    target = self.sel_focus.clone();
                    self.ime = None;
                    self.ime_cursor_area = Rect::ZERO;
                }
                if let Some(id) = target {
                    self.send_event(widget, id, Event::LostImeFocus);
                }
            }
            winit::event::Ime::Preedit(text, cursor) => {
                if self.ime.is_some()
                    && let Some(id) = self.sel_focus.clone()
                {
                    self.send_event(widget, id, Event::ImePreedit(&text, cursor));
                }
            }
            winit::event::Ime::Commit(text) => {
                if self.ime.is_some()
                    && let Some(id) = self.sel_focus.clone()
                {
                    self.send_event(widget, id, Event::ImeCommit(&text));
                }
            }
        }
    }

    // Set selection focus to `wid` immediately; if `key_focus` also set that
    pub(super) fn set_sel_focus(
        &mut self,
        window: &dyn WindowDataErased,
        mut widget: Node<'_>,
        pending: PendingSelFocus,
    ) {
        let PendingSelFocus {
            target,
            key_focus,
            ime,
            source,
        } = pending;
        let target_is_new = target != self.sel_focus;
        let old_key_focus = self.key_focus;
        self.key_focus = key_focus;

        log::trace!("set_sel_focus: target={target:?}, key_focus={key_focus}");

        if let Some(id) = self.sel_focus.clone() {
            if self.ime.is_some() && (ime.is_none() || target_is_new) {
                window.set_ime_allowed(None);
                self.old_ime_target = Some(id.clone());
                self.ime = None;
                self.ime_cursor_area = Rect::ZERO;
            }

            if old_key_focus && (!key_focus || target_is_new) {
                // If widget has key focus, this is lost
                self.send_event(widget.re(), id.clone(), Event::LostKeyFocus);
            }

            if target.is_none() {
                // Retain selection focus without a new target
                return;
            } else if target_is_new {
                // Selection focus is lost if another widget receives key focus
                self.send_event(widget.re(), id, Event::LostSelFocus);
            }
        }

        if let Some(id) = target.clone() {
            if target_is_new {
                self.send_event(widget.re(), id.clone(), Event::SelFocus(source));
            }

            if key_focus && (!old_key_focus || target_is_new) {
                self.send_event(widget.re(), id.clone(), Event::KeyFocus);
            }

            if ime.is_some() && (ime != self.ime || target_is_new) {
                window.set_ime_allowed(ime);
                self.ime = ime;
            }
        }

        self.sel_focus = target;
    }
}
