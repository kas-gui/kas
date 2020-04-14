// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use crate::event::{self, Event, Manager, Response};
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
    /// [`Event::Activate`] as appropriate (on primary mouse button only).
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
    fn action(&mut self, _: &mut Manager, event: Event) -> Response<Self::Msg> {
        Response::Unhandled(event)
    }
}

/// Event routing
///
/// This trait is responsible for routing events to the correct widget. It is
/// separate from [`Handler`] since it can be derived for many parent widgets,
/// even when event *handling* must be implemented manually.
///
/// This trait is implemented by `derive(Widget)` when a `#[handler]` attribute
/// is present with parameter `send` or `all`.
pub trait SendEvent: Handler {
    /// Send an event
    ///
    /// This method is responsible for routing events toward descendents.
    /// [`WidgetId`] values are assigned via depth-first search with parents
    /// ordered after all children.
    /// The following logic is recommended for routing events:
    /// ```norun
    /// if id <= self.child1.id() {
    ///     self.child1.event(mgr, id, event).into()
    /// } else if id <= self.child2.id() {
    ///     self.child2.event(mgr, id, event).into()
    /// } ... {
    /// } else {
    ///     debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
    ///     Manager::handle_generic(self, mgr, event)
    /// }
    /// ```
    ///
    /// When the child's [`Handler::Msg`] type is not [`VoidMsg`], its response
    /// messages can be handled here (in place of `.into()` above).
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg>;
}

impl<'a> Manager<'a> {
    /// Generic handler for low-level events passed to leaf widgets
    pub fn handle_generic<W>(
        widget: &mut W,
        mgr: &mut Manager,
        mut event: Event,
    ) -> Response<<W as Handler>::Msg>
    where
        W: Handler + ?Sized,
    {
        if widget.is_disabled() {
            return Response::Unhandled(event);
        }

        if widget.activation_via_press() {
            // Translate press events
            match event {
                Event::PressStart { source, coord } if source.is_primary() => {
                    mgr.request_grab(widget.id(), source, coord, event::GrabMode::Grab, None);
                    return Response::None;
                }
                Event::PressMove { .. } => {
                    // We don't need these events, but they should not be considered *unhandled*
                    return Response::None;
                }
                Event::PressEnd { end_id, .. } if end_id == Some(widget.id()) => {
                    event = Event::Activate;
                }
                _ => (),
            };
        }
        widget.action(mgr, event)
    }
}
