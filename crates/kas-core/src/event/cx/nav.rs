// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context: navigation focus

use super::{EventCx, EventState};
use crate::event::{Event, FocusSource};
use crate::{Action, Id, NavAdvance, Node};
#[allow(unused)] use crate::{Tile, event::Command};

#[crate::impl_default(PendingNavFocus::None)]
pub(super) enum PendingNavFocus {
    None,
    Set {
        target: Option<Id>,
        source: FocusSource,
    },
    Next {
        target: Option<Id>,
        reverse: bool,
        source: FocusSource,
    },
}

impl EventState {
    /// Get whether this widget has navigation focus
    #[inline]
    pub fn has_nav_focus(&self, w_id: &Id) -> bool {
        *w_id == self.nav_focus
    }

    /// Get the current navigation focus, if any
    ///
    /// This is the widget selected by navigating the UI with the Tab key.
    ///
    /// Note: changing navigation focus (e.g. via [`Self::clear_nav_focus`],
    /// [`Self::set_nav_focus`] or [`Self::next_nav_focus`]) does not
    /// immediately affect the result of this method.
    #[inline]
    pub fn nav_focus(&self) -> Option<&Id> {
        self.nav_focus.as_ref()
    }

    /// Clear navigation focus
    pub fn clear_nav_focus(&mut self) {
        self.pending_nav_focus = PendingNavFocus::Set {
            target: None,
            source: FocusSource::Synthetic,
        };
    }

    pub(super) fn clear_nav_focus_on(&mut self, target: &Id) {
        if let Some(id) = self.nav_focus.as_ref()
            && target.is_ancestor_of(id)
        {
            if matches!(&self.pending_nav_focus, PendingNavFocus::Set { target, .. } if target.as_ref() == Some(id))
            {
                self.pending_nav_focus = PendingNavFocus::None;
            }

            if matches!(self.pending_nav_focus, PendingNavFocus::None) {
                self.pending_nav_focus = PendingNavFocus::Set {
                    target: None,
                    source: FocusSource::Synthetic,
                };
            }
        }
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
    pub fn set_nav_focus(&mut self, id: Id, source: FocusSource) {
        self.pending_nav_focus = PendingNavFocus::Set {
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
        self.pending_nav_focus = PendingNavFocus::Next {
            target: target.into(),
            reverse,
            source,
        };
    }

    /// Attempts to set a fallback to receive [`Event::Command`]
    ///
    /// In case a navigation key is pressed (see [`Command`]) but no widget has
    /// navigation focus, then, if a fallback has been set, that widget will
    /// receive the key via [`Event::Command`].
    ///
    /// Only one widget can be a fallback, and the *first* to set itself wins.
    /// This is primarily used to allow scroll-region widgets to
    /// respond to navigation keys when no widget has focus.
    pub fn register_nav_fallback(&mut self, id: Id) {
        if self.nav_fallback.is_none() {
            log::debug!(target: "kas_core::event","register_nav_fallback: id={id}");
            self.nav_fallback = Some(id);
        }
    }
}

impl<'a> EventCx<'a> {
    // Call Widget::_nav_next
    #[inline]
    pub(super) fn nav_next(
        &mut self,
        mut widget: Node<'_>,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id> {
        log::trace!(target: "kas_core::event", "nav_next: focus={focus:?}, advance={advance:?}");

        widget._nav_next(&mut self.config_cx(), focus, advance)
    }

    /// Set navigation focus immediately
    pub(super) fn set_nav_focus_impl(
        &mut self,
        mut widget: Node,
        target: Option<Id>,
        source: FocusSource,
    ) {
        if target == self.nav_focus || !self.config.nav_focus {
            return;
        }

        self.clear_key_focus();

        if let Some(old) = self.nav_focus.take() {
            self.action(&old, Action::REDRAW);
            self.send_event(widget.re(), old, Event::LostNavFocus);
        }

        self.nav_focus = target.clone();
        log::debug!(target: "kas_core::event", "nav_focus = {target:?}");
        if let Some(id) = target {
            self.action(&id, Action::REDRAW);
            self.send_event(widget, id, Event::NavFocus(source));
        }
    }

    /// Advance the keyboard navigation focus immediately
    pub(super) fn next_nav_focus_impl(
        &mut self,
        mut widget: Node,
        target: Option<Id>,
        reverse: bool,
        source: FocusSource,
    ) {
        if !self.config.nav_focus || (target.is_some() && target == self.nav_focus) {
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
                self.next_nav_focus_impl(node, target, reverse, source)
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

        let advance = if !reverse {
            NavAdvance::Forward(target.is_some())
        } else {
            NavAdvance::Reverse(target.is_some())
        };
        let focus = target.or_else(|| self.nav_focus.clone());

        // Whether to restart from the beginning on failure
        let restart = focus.is_some();

        let mut opt_id = self.nav_next(widget.re(), focus.as_ref(), advance);
        if restart && opt_id.is_none() {
            opt_id = self.nav_next(widget.re(), None, advance);
        }

        self.set_nav_focus_impl(widget, opt_id, source);
    }
}
