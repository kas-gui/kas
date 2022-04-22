// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use super::*;
#[allow(unused)]
use crate::Widget; // for doc-links
use crate::{WidgetConfig, WidgetExt, WidgetId};
use kas_macros::autoimpl;

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
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Handler: WidgetConfig {
    /// Generic handler: translate presses to activations
    ///
    /// This is configuration for [`EventMgr::handle_generic`], and can be used
    /// to translate *press* (click/touch) events into [`Event::Activate`].
    // NOTE: not an associated constant because these are not object-safe
    #[inline]
    fn activation_via_press(&self) -> bool {
        false
    }

    /// Generic handler: focus rect on key navigation
    ///
    /// If this widget receives [`Event::NavFocus`]`(true)` then return
    /// [`Response::Focus`] with the widget's rect. By default this is true if
    /// and only if [`WidgetConfig::key_nav`] is true.
    #[inline]
    fn focus_on_key_nav(&self) -> bool {
        self.key_nav()
    }

    /// Handle an event and return a user-defined message
    ///
    /// Widgets should handle any events applicable to themselves here, and
    /// return all other events via [`Response::Unused`].
    #[inline]
    fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
        let _ = (mgr, event);
        Response::Unused
    }

    /// Handler for messages from children/descendants
    ///
    /// This method is called when a child leaves a message on the stack. *Some*
    /// parent or ancestor widget should read this message.
    ///
    /// Any [`Response`] value may be returned. In normal usage, `Used` or
    /// `Unused` is returned (the distinction is unimportant).
    ///
    /// The default implementation does nothing.
    #[inline]
    fn on_message(&mut self, mgr: &mut EventMgr, index: usize) -> Response {
        let _ = (mgr, index);
        Response::Unused
    }

    /// Handler for scrolling
    ///
    /// Called when [`Response::Pan`], [`Response::Scrolled`] or
    /// [`Response::Focus`] is emitted, including when emitted by self.
    ///
    /// The default implementation simply returns `scroll`.
    #[inline]
    fn scroll(&mut self, mgr: &mut EventMgr, scroll: Scroll) -> Scroll {
        let _ = mgr;
        scroll
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
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait SendEvent: Handler {
    /// Send an event
    ///
    /// This method is responsible for routing events toward descendants.
    /// [`WidgetId`] values are assigned via depth-first search with parents
    /// ordered after all children.
    ///
    /// The following logic is recommended for routing events:
    /// ```no_test
    /// match self.find_child_index(&id) {
    ///     Some(widget_index![self.child0]) => self.child0.send(mgr, id, event).into(),
    ///     Some(widget_index![self.child1]) => self.child1.send(mgr, id, event).into(),
    ///     // ...
    ///     _ => {
    ///         debug_assert_eq!(self.id(), id);
    ///         EventMgr::handle_generic(self, mgr, event),
    ///     }
    /// }
    /// ```
    ///
    /// When the child's [`Handler::Msg`] type is not something which converts
    /// into the widget's own message type, it must be handled here (in place of `.into()`).
    ///
    /// The example above uses [`EventMgr::handle_generic`], which is an optional
    /// tool able to perform some simplifications on events. It is also valid to
    /// call [`Handler::handle`] directly or simply to embed handling logic here.
    ///
    /// When a child widget returns [`Response::Unused`], the widget may call
    /// its own event handler. This is useful e.g. to capture a click+drag on a
    /// child which does not handle that event. Note further that in case the
    /// child is disabled, events targetting the child may be sent directly to
    /// self.
    fn send(&mut self, mgr: &mut EventMgr, id: WidgetId, event: Event) -> Response;
}

impl<'a> EventMgr<'a> {
    /// Generic event simplifier
    ///
    /// This is a free function often called from [`SendEvent::send`] to
    /// simplify certain events and then invoke [`Handler::handle`].
    pub fn handle_generic<W>(widget: &mut W, mgr: &mut EventMgr, mut event: Event) -> Response
    where
        W: Handler + ?Sized,
    {
        if widget.activation_via_press() {
            // Translate press events
            match event {
                Event::PressStart { source, coord, .. } if source.is_primary() => {
                    mgr.grab_press(widget.id(), source, coord, GrabMode::Grab, None);
                    return Response::Used;
                }
                Event::PressMove { source, cur_id, .. } => {
                    let cond = widget.eq_id(&cur_id);
                    let target = if cond { cur_id } else { None };
                    mgr.set_grab_depress(source, target);
                    return Response::Used;
                }
                Event::PressEnd {
                    end_id, success, ..
                } if success && widget.eq_id(&end_id) => {
                    event = Event::Activate;
                }
                Event::PressEnd { .. } => return Response::Used,
                _ => (),
            };
        }

        if widget.focus_on_key_nav() && event == Event::NavFocus(true) {
            mgr.set_scroll(Scroll::Rect(widget.rect()));
        }

        widget.handle(mgr, event)
    }
}
