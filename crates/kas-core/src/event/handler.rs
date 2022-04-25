// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use super::*;
#[allow(unused)]
use crate::Widget; // for doc-links
use crate::{WidgetConfig, WidgetExt};
use kas_macros::autoimpl;

/// Event handling for a [`Widget`]
///
/// This trait is part of the [`Widget`] family and is derived by
/// [`derive(Widget)`] unless `#[handler(handle = noauto)]`
/// or `#[handler(noauto)]` is used.
///
/// [`derive(Widget)`]: ../macros/index.html#the-derivewidget-macro
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Handler: WidgetConfig {
    /// Generic handler: translate presses to activations
    ///
    /// If true, [`Event::PressStart`] (and other press events) will not be sent
    /// to [`Handler::handle_event`]; instead [`Event::Activate`] will be sent on
    /// "click events".
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

    /// Handle an event sent to this widget
    ///
    /// An [`Event`] is some form of user input, timer or notification.
    ///
    /// This is the primary event handler for a widget. Secondary handlers are:
    ///
    /// -   If this method returns [`Response::Unused`], then
    ///     [`Handler::handle_unused`] is called on each parent until the event
    ///     is used (or the root widget is reached)
    /// -   If a message is left on the stack by [`EventMgr::push_msg`], then
    ///     [`Handler::handle_message`] is called on each parent until the stack is
    ///     empty (failing to empty the stack results in a warning in the log).
    /// -   If any scroll state is set by [`EventMgr::set_scroll`], then
    ///     [`Handler::scroll`] is called for each parent
    ///
    /// Default implementation: do nothing; return [`Response::Unused`].
    #[inline]
    fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
        let _ = (mgr, event);
        Response::Unused
    }

    /// Handle an event sent to child `index` but left unhandled
    ///
    /// Default implementation: call [`Self::handle_event`] with `event`.
    #[inline]
    fn handle_unused(&mut self, mgr: &mut EventMgr, index: usize, event: Event) -> Response {
        let _ = index;
        self.handle_event(mgr, event)
    }

    /// Handler for messages from children/descendants
    ///
    /// This method is called when a child leaves a message on the stack. *Some*
    /// parent or ancestor widget should read this message.
    ///
    /// The default implementation does nothing.
    #[inline]
    fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
        let _ = (mgr, index);
    }

    /// Handler for scrolling
    ///
    /// This is the last "event handling step" for each widget. If
    /// [`Self::handle_event`], [`Self::handle_unused`], [`Self::handle_message`] or any
    /// child's event handlers set a non-empty scroll value
    /// (via [`EventMgr::set_scroll`]), this gets called and the result set as
    /// the new scroll value.
    ///
    /// If [`Layout::translation`] is non-zero and `scroll` is
    /// `Scroll::Rect(_)`, then this method should undo the translation.
    ///
    /// The default implementation simply returns `scroll`.
    #[inline]
    fn handle_scroll(&mut self, mgr: &mut EventMgr, scroll: Scroll) -> Scroll {
        let _ = mgr;
        scroll
    }
}

impl<'a> EventMgr<'a> {
    /// Generic event simplifier
    pub(crate) fn handle_generic(&mut self, widget: &mut dyn Widget, mut event: Event) -> Response {
        if widget.activation_via_press() {
            // Translate press events
            match event {
                Event::PressStart { source, coord, .. } if source.is_primary() => {
                    self.grab_press(widget.id(), source, coord, GrabMode::Grab, None);
                    return Response::Used;
                }
                Event::PressMove { source, cur_id, .. } => {
                    let cond = widget.eq_id(&cur_id);
                    let target = if cond { cur_id } else { None };
                    self.set_grab_depress(source, target);
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

        let mut response = Response::Unused;
        if widget.focus_on_key_nav() && event == Event::NavFocus(true) {
            self.set_scroll(Scroll::Rect(widget.rect()));
            response = Response::Used;
        }

        response | widget.handle_event(self, event)
    }
}
