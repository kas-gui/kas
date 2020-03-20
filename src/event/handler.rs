// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use crate::event::{self, Action, Event, Manager, Response};
use crate::Widget;

/// High-level event handling for a [`Widget`]
///
/// This is a companion trait to [`Widget`]. It can (optionally) be implemented
/// by the `derive(Widget)` macro, or can be implemented manually.
pub trait Handler: Widget {
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
