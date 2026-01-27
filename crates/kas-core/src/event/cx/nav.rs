// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context: navigation focus

use super::{ConfigCx, EventCx, EventState};
use crate::event::{Event, FocusSource};
use crate::{Id, Node};
#[allow(unused)] use crate::{Tile, event::Command};

/// Action of Widget::_nav_next
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NavAdvance {
    /// Match only `focus` if navigable
    None,
    /// Walk children forwards, self first
    ///
    /// Parameter: whether this can match self (in addition to other widgets).
    Forward(bool),
    /// Walk children backwards, self last
    ///
    /// Parameter: whether this can match self (in addition to other widgets).
    Reverse(bool),
}

#[crate::impl_default(PendingNavFocus::None)]
pub(super) enum PendingNavFocus {
    None,
    Set {
        target: Option<Id>,
        source: FocusSource,
    },
    Next {
        target: Option<Id>,
        advance: NavAdvance,
        source: FocusSource,
    },
}

#[derive(Default)]
pub(super) struct NavFocus {
    focus: Option<Id>,
    pub(super) fallback: Option<Id>,
    pending_focus: PendingNavFocus,
}

impl NavFocus {
    #[inline]
    pub(super) fn has_pending_changes(&self) -> bool {
        !matches!(self.pending_focus, PendingNavFocus::None)
    }
}

impl EventState {
    /// Get whether this widget has navigation focus
    #[inline]
    pub fn has_nav_focus(&self, w_id: &Id) -> bool {
        *w_id == self.nav.focus
    }

    /// Get the current navigation focus, if any
    ///
    /// This is the widget selected by navigating the UI with the Tab key.
    ///
    /// Note: changing navigation focus (e.g. via [`Self::clear_nav_focus`],
    /// [`Self::request_nav_focus`] or [`Self::next_nav_focus`]) does not
    /// immediately affect the result of this method.
    #[inline]
    pub fn nav_focus(&self) -> Option<&Id> {
        self.nav.focus.as_ref()
    }

    /// Clear navigation focus
    pub fn clear_nav_focus(&mut self) {
        self.nav.pending_focus = PendingNavFocus::Set {
            target: None,
            source: FocusSource::Synthetic,
        };
    }

    pub(super) fn clear_nav_focus_on(&mut self, target: &Id) {
        if let Some(id) = self.nav.focus.as_ref()
            && target.is_ancestor_of(id)
        {
            if matches!(&self.nav.pending_focus, PendingNavFocus::Set { target, .. } if target.as_ref() == Some(id))
            {
                self.nav.pending_focus = PendingNavFocus::None;
            }

            if matches!(self.nav.pending_focus, PendingNavFocus::None) {
                self.nav.pending_focus = PendingNavFocus::Set {
                    target: None,
                    source: FocusSource::Synthetic,
                };
            }
        }
    }

    /// Request navigation focus directly
    ///
    /// If `id` already has navigation focus or navigation focus is disabled
    /// globally then nothing happens. If widget `id` supports
    /// [navigation focus](Tile::navigable), then it should receive
    /// [`Event::NavFocus`]; if not then the first supporting ancestor will
    /// receive focus.
    pub fn request_nav_focus(&mut self, id: Id, source: FocusSource) {
        self.nav.pending_focus = PendingNavFocus::Next {
            target: Some(id),
            advance: NavAdvance::None,
            source,
        };
    }

    /// Set navigation focus directly
    ///
    /// If `id` already has navigation focus or navigation focus is disabled
    /// globally then nothing happens, otherwise widget `id` should receive
    /// [`Event::NavFocus`].
    ///
    /// Normally, [`Tile::navigable`] will return true for widget `id` but this
    /// is not checked or required. For example, a `ScrollLabel` can receive
    /// focus on text selection with the mouse.
    pub(crate) fn set_nav_focus(&mut self, id: Id, source: FocusSource) {
        self.nav.pending_focus = PendingNavFocus::Set {
            target: Some(id),
            source,
        };
    }

    /// Advance the navigation focus
    ///
    /// If `target == Some(id)`, this looks for the next widget from `id`
    /// (inclusive) which is [navigable](Tile::navigable). Otherwise where
    /// some widget `id` has [`nav_focus`](Self::nav_focus) this looks for the
    /// next navigable widget *excluding* `id`. If no reference is available,
    /// this instead looks for the first navigable widget.
    ///
    /// If `reverse`, instead search for the previous or last navigable widget.
    pub fn next_nav_focus(
        &mut self,
        target: impl Into<Option<Id>>,
        reverse: bool,
        source: FocusSource,
    ) {
        let target = target.into();
        let advance = match reverse {
            false => NavAdvance::Forward(target.is_some()),
            true => NavAdvance::Reverse(target.is_some()),
        };
        self.nav.pending_focus = PendingNavFocus::Next {
            target,
            advance,
            source,
        };
    }
}

impl<'a> ConfigCx<'a> {
    /// Sets the fallback recipient of [`Event::Command`]
    ///
    /// Where a key-press translates to a [`Command`], this is first sent to
    /// widgets with applicable key, selection and/or navigation focus as an
    /// [`Event::Command`]. If this event goes unhandled and a fallback
    /// recipient is set using this method, then this fallback recipient will
    /// be sent the same event.
    ///
    /// There may be one fallback recipient per window; do not use an [`Id`]
    /// from another window. If this method is called multiple times, the last
    /// such call succeeds.
    pub fn register_nav_fallback(&mut self, id: Id) {
        if self.nav.fallback.is_none() {
            log::debug!(target: "kas_core::event","register_nav_fallback: id={id}");
            self.nav.fallback = Some(id);
        }
    }
}

impl<'a> EventCx<'a> {
    // Call Widget::_nav_next
    #[inline]
    pub(super) fn nav_next(
        &mut self,
        tile: &dyn Tile,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id> {
        log::trace!(target: "kas_core::event", "nav_next: focus={focus:?}, advance={advance:?}");

        tile._nav_next(self, focus, advance)
    }

    pub(super) fn handle_pending_nav_focus(&mut self, widget: Node<'_>) {
        match std::mem::take(&mut self.nav.pending_focus) {
            PendingNavFocus::None => (),
            PendingNavFocus::Set { target, source } => {
                self.set_nav_focus_impl(widget, target, source)
            }
            PendingNavFocus::Next {
                target,
                advance,
                source,
            } => self.next_nav_focus_impl(widget, target, advance, source),
        }
    }

    /// Set navigation focus immediately
    pub(super) fn set_nav_focus_impl(
        &mut self,
        mut widget: Node,
        target: Option<Id>,
        source: FocusSource,
    ) {
        if target == self.nav.focus || !self.config.nav_focus {
            return;
        }

        if let Some(id) = self.input.sel_focus().cloned()
            && id != target
        {
            self.input.clear_sel_socus_on(&id);
        }

        if let Some(old) = self.nav.focus.take() {
            self.redraw();
            self.send_event(widget.re(), old, Event::LostNavFocus);
        }

        self.nav.focus = target.clone();
        log::debug!(target: "kas_core::event", "nav_focus = {target:?}");
        if let Some(id) = target {
            self.redraw();
            self.send_event(widget, id, Event::NavFocus(source));
        }
    }

    /// Advance the keyboard navigation focus immediately
    pub(super) fn next_nav_focus_impl(
        &mut self,
        mut widget: Node,
        target: Option<Id>,
        advance: NavAdvance,
        source: FocusSource,
    ) {
        if !self.config.nav_focus || (target.is_some() && target == self.nav.focus) {
            return;
        }

        if let Some(id) = self
            .popups
            .last()
            .filter(|popup| popup.is_sized)
            .map(|state| state.desc.id.clone())
        {
            if id.is_ancestor_of(widget.id_ref()) {
                // do nothing
            } else if let Some(r) = widget.find_node(&id, |node| {
                self.next_nav_focus_impl(node, target, advance, source)
            }) {
                return r;
            } else {
                log::warn!(
                    target: "kas_core::event",
                    "next_nav_focus: have open pop-up which is not a child of widget",
                );
                return;
            }
        }

        let focus = target.or_else(|| self.nav.focus.clone());

        // Whether to restart from the beginning on failure
        let restart = focus.is_some();

        let mut opt_id = self.nav_next(widget.as_tile(), focus.as_ref(), advance);
        if restart && opt_id.is_none() {
            opt_id = self.nav_next(widget.as_tile(), None, advance);
        }

        self.set_nav_focus_impl(widget, opt_id, source);
    }
}
