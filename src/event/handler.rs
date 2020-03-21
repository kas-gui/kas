// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use crate::event::{self, Action, Event, Manager, Response};
#[allow(unused)]
use crate::Widget; // for doc-links
use crate::{WidgetConfig, WidgetId};

/// High-level event handling for a [`Widget`]
///
/// This is implemented by `derive(Widget)` when a `#[handler]` attribute is
/// present with parameter `action` or `all`.
pub trait Handler: WidgetConfig {
    /// Type of message returned by this handler.
    ///
    /// This mechanism allows type-safe handling of user-defined responses to handled actions.
    /// For example, a user may define a control panel where each button returns a unique code,
    /// or a configuration editor may return a full copy of the new configuration on completion.
    type Msg;

    /// Configuration for [`Manager::handle_generic`]
    ///
    /// If this returns true, then click/touch events get translated to
    /// [`Action::Activate`] as appropriate (on primary mouse button only).
    // NOTE: not an associated constant because these are not object-safe
    #[inline]
    fn activation_via_press(&self) -> bool {
        false
    }

    /// Handle a high-level "action" and return a user-defined message.
    ///
    /// Widgets should handle any events applicable to themselves here, and
    /// return all other events via [`Response::Unhandled`].
    #[inline]
    fn action(&mut self, _: &mut Manager, action: Action) -> Response<Self::Msg> {
        Response::Unhandled(Event::Action(action))
    }
}

/// Low-level event handling for a [`Widget`]
///
/// This is implemented by `derive(Widget)` when a `#[handler]` attribute is
/// present with parameter `event` or `all`.
pub trait EventHandler: Handler {
    /// Handle a low-level event.
    ///
    /// Most non-parent widgets will not need to implement this method manually.
    /// The default implementation (which wraps [`Manager::handle_generic`])
    /// forwards high-level events via [`event::Handler::action`], thus the only
    /// reason for non-parent widgets to implement this manually is for
    /// low-level event processing.
    ///
    /// Parent widgets should forward events to the appropriate child widget,
    /// via logic like the following:
    /// ```norun
    /// if id <= self.child1.id() {
    ///     self.child1.event(mgr, id, event).into()
    /// } else if id <= self.child2.id() {
    ///     self.child2.event(mgr, id, event).into()
    /// } else {
    ///     debug_assert!(id == self.id(), "Layout::event: bad WidgetId");
    ///     // either handle `event`, or return:
    ///     Response::Unhandled(event)
    /// }
    /// ```
    /// Optionally, the return value of child event handlers may be intercepted
    /// in order to handle returned messages and/or unhandled events.
    #[inline]
    fn event(&mut self, mgr: &mut Manager, _: WidgetId, event: Event) -> Response<Self::Msg> {
        Manager::handle_generic(self, mgr, event)
    }
}

impl<'a> Manager<'a> {
    /// Generic handler for low-level events passed to leaf widgets
    pub fn handle_generic<W>(
        widget: &mut W,
        mgr: &mut Manager,
        event: Event,
    ) -> Response<<W as Handler>::Msg>
    where
        W: Handler + ?Sized,
    {
        let activable = widget.activation_via_press();
        match event {
            Event::Action(action) => widget.action(mgr, action),
            Event::PressStart { source, coord } if activable && source.is_primary() => {
                mgr.request_grab(widget.id(), source, coord, event::GrabMode::Grab, None);
                Response::None
            }
            Event::PressMove { .. } if activable => {
                // We don't need these events, but they should not be considered *unhandled*
                Response::None
            }
            Event::PressEnd { end_id, .. } if activable && end_id == Some(widget.id()) => {
                widget.action(mgr, Action::Activate)
            }
            ev @ _ => Response::Unhandled(ev),
        }
    }
}
