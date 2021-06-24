// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use super::*;
#[allow(unused)]
use crate::Widget; // for doc-links
use crate::{WidgetConfig, WidgetId};

/// Event handling for a [`Widget`]
///
/// This trait is part of the [`Widget`] family and is derived by
/// [`derive(Widget)`] unless `#[handler(handle = noauto)]`
/// or `#[handler(noauto)]` is used.
///
/// Interactive widgets should implement their event-handling logic here
/// (although it is also possible to implement this in [`SendEvent::send`],
/// which might be preferable when dealing with child widgets).
///
/// The default implementation does nothing, and is derived by `derive(Widget)`
/// when a `#[handler]` attribute is present (except with parameter
/// `handler=noauto`).
///
/// [`derive(Widget)`]: ../macros/index.html#the-derivewidget-macro
pub trait Handler: WidgetConfig {
    /// Type of message returned by this widget
    ///
    /// This mechanism allows type-safe handling of user-defined responses to
    /// handled actions, for example an enum encoding button presses or a
    /// floating-point value from a slider.
    ///
    /// The [`VoidMsg`] type may be used where messages are never generated.
    /// This is distinct from `()`, which might be applicable when a widget only
    /// needs to "wake up" a parent.
    type Msg;

    /// Generic handler: translate presses to activations
    ///
    /// This is configuration for [`Manager::handle_generic`], and can be used
    /// to translate *press* (click/touch) events into [`Event::Activate`].
    // NOTE: not an associated constant because these are not object-safe
    #[inline]
    fn activation_via_press(&self) -> bool {
        false
    }

    /// Handle an event and return a user-defined message
    ///
    /// Widgets should handle any events applicable to themselves here, and
    /// return all other events via [`Response::Unhandled`].
    #[inline]
    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        let _ = (mgr, event);
        Response::Unhandled
    }
}

/// Event routing
///
/// This trait is part of the [`Widget`] family and is derived by
/// [`derive(Widget)`] unless `#[handler(send = noauto)]`
/// or `#[handler(noauto)]` is used.
///
/// This trait is responsible for routing events to the correct widget. It is
/// separate from [`Handler`] since it can be derived for many parent widgets,
/// even when event *handling* must be implemented manually.
///
/// This trait is implemented by `derive(Widget)` when a `#[handler]` attribute
/// is present (except with parameter `send=noauto`).
///
/// [`derive(Widget)`]: ../macros/index.html#the-derivewidget-macro
pub trait SendEvent: Handler {
    /// Send an event
    ///
    /// This method is responsible for routing events toward descendents.
    /// [`WidgetId`] values are assigned via depth-first search with parents
    /// ordered after all children. Disabling a widget is recursive, hence
    /// disabled widgets should not forward any events.
    ///
    /// The following logic is recommended for routing events:
    /// ```no_test
    /// if self.is_disabled() {
    ///     return Response::Unhandled;
    /// }
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
    /// Parents which don't handle any events themselves may simplify this:
    /// ```no_test
    /// if !self.is_disabled() && id <= self.w.id() {
    ///     return self.w.send(mgr, id, event);
    /// }
    /// Response::Unhandled
    /// ```
    ///
    /// When the child's [`Handler::Msg`] type is not [`VoidMsg`], its response
    /// messages can be handled here (in place of `.into()` above).
    ///
    /// The example above uses [`Manager::handle_generic`], which is an optional
    /// tool able to perform some simplifications on events. It is also valid to
    /// call [`Handler::handle`] directly or simply to embed handling logic here.
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg>;
}

impl<'a> Manager<'a> {
    /// Generic event simplifier
    ///
    /// This is a free function often called from [`SendEvent::send`] to
    /// simplify certain events and then invoke [`Handler::handle`].
    pub fn handle_generic<W>(
        widget: &mut W,
        mgr: &mut Manager,
        mut event: Event,
    ) -> Response<<W as Handler>::Msg>
    where
        W: Handler + ?Sized,
    {
        if widget.activation_via_press() {
            // Translate press events
            match event {
                Event::PressStart { source, coord, .. } if source.is_primary() => {
                    mgr.request_grab(widget.id(), source, coord, GrabMode::Grab, None);
                    return Response::None;
                }
                Event::PressMove { source, cur_id, .. } => {
                    let cond = cur_id == Some(widget.id());
                    let target = if cond { cur_id } else { None };
                    mgr.set_grab_depress(source, target);
                    return Response::None;
                }
                Event::PressEnd { end_id, .. } if end_id == Some(widget.id()) => {
                    event = Event::Activate;
                }
                _ => (),
            };
        }
        if event == Event::NavFocus {
            return Response::Focus(widget.rect());
        }
        widget.handle(mgr, event)
    }
}
