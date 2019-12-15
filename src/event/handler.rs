// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use crate::event::{Action, Event, EventChild, Manager, Response};
use crate::{TkWindow, Widget};

/// Event-handling aspect of a widget.
///
/// This is a companion trait to [`Widget`]. It can (optionally) be implemented
/// by the `derive(Widget)` macro, or can be implemented manually.
///
/// [`Widget`]: crate::Widget
pub trait Handler: Widget {
    /// Type of message returned by this handler.
    ///
    /// This mechanism allows type-safe handling of user-defined responses to handled actions.
    /// For example, a user may define a control panel where each button returns a unique code,
    /// or a configuration editor may return a full copy of the new configuration on completion.
    type Msg;

    /// Handle a high-level "action" and return a user-defined message.
    ///
    /// Widgets should handle any events applicable to themselves here, and
    /// return all other events via [`Response::Unhandled`].
    #[inline]
    fn handle_action(&mut self, _: &mut dyn TkWindow, action: Action) -> Response<Self::Msg> {
        Response::Unhandled(EventChild::Action(action))
    }

    /// Handle a low-level event.
    ///
    /// Most non-parent widgets will not need to implement this method manually.
    /// The default implementation (which wraps [`Manager::handle_generic`])
    /// forwards high-level events via [`Handler::handle_action`].
    ///
    /// Parent widgets should forward events to the appropriate child widget,
    /// translating event coordinates where applicable. Unused events should be
    /// handled (directly or through [`Manager::handle_generic`]) or returned
    /// via [`Response::Unhandled`]. The return-value from child handlers may
    /// be intercepted in order to handle as-yet-unhandled events.
    ///
    /// Additionally, this method allows lower-level interpretation of some
    /// events, e.g. more direct access to mouse inputs.
    #[inline]
    fn handle(&mut self, tk: &mut dyn TkWindow, event: Event) -> Response<Self::Msg> {
        Manager::handle_generic(self, tk, event)
    }
}

impl Manager {
    /// Generic handler for low-level events passed to leaf widgets
    pub fn handle_generic<W>(
        widget: &mut W,
        tk: &mut dyn TkWindow,
        event: Event,
    ) -> Response<<W as Handler>::Msg>
    where
        W: Handler + ?Sized,
    {
        match event {
            Event::ToChild(_, ev) | Event::ToCoord(_, ev) => match ev {
                EventChild::Action(action) => widget.handle_action(tk, action),
                EventChild::Identify => Response::Identify(widget.id()),
                ev @ _ => Response::Unhandled(ev),
            },
        }
    }

    /// Handler for low-level events passed to leaf widgets, supporting
    /// activation via mouse and touch.
    // TODO: this will likely be replaced with some more generic handler
    pub fn handle_activable<W>(
        widget: &mut W,
        tk: &mut dyn TkWindow,
        event: Event,
    ) -> Response<<W as Handler>::Msg>
    where
        W: Handler + ?Sized,
    {
        match event {
            Event::ToChild(_, ev) | Event::ToCoord(_, ev) => match ev {
                EventChild::Action(action) => widget.handle_action(tk, action),
                EventChild::Identify => Response::Identify(widget.id()),
                EventChild::PressStart { source, coord } if source.is_primary() => {
                    tk.update_data(&mut |data| data.request_press_grab(source, widget.id(), coord));
                    Response::None
                }
                EventChild::PressEnd { start_id, .. } if start_id == Some(widget.id()) => {
                    widget.handle_action(tk, Action::Activate)
                }
                ev @ _ => Response::Unhandled(ev),
            },
        }
    }
}
