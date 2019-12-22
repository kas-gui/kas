// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling - handler

use crate::event::{Action, Address, Event, Manager, Response};
use crate::geom::Rect;
use crate::layout::{AxisInfo, SizeRules};
use crate::theme::{DrawHandle, SizeHandle};
use crate::{CoreData, TkWindow, Widget, WidgetCore};

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
    fn handle_action(&mut self, _: &mut dyn TkWindow, action: Action) -> Response<Self::Msg> {
        Response::Unhandled(Event::Action(action))
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
    fn handle(&mut self, tk: &mut dyn TkWindow, _: Address, event: Event) -> Response<Self::Msg> {
        Manager::handle_generic(self, tk, event)
    }
}

// These implementations are somewhat redundant with Box<dyn Widget>.
// TODO: do we want to keep both?
impl<M> Handler for Box<dyn Handler<Msg = M>> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        self.as_ref().activation_via_press()
    }

    #[inline]
    fn handle_action(&mut self, tk: &mut dyn TkWindow, action: Action) -> Response<Self::Msg> {
        self.as_mut().handle_action(tk, action)
    }

    #[inline]
    fn handle(
        &mut self,
        tk: &mut dyn TkWindow,
        addr: Address,
        event: Event,
    ) -> Response<Self::Msg> {
        self.as_mut().handle(tk, addr, event)
    }
}

impl<M> Widget for Box<dyn Handler<Msg = M>> {
    fn allow_focus(&self) -> bool {
        self.as_ref().allow_focus()
    }

    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.as_mut().size_rules(size_handle, axis)
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        self.as_mut().set_rect(size_handle, rect);
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &Manager) {
        self.as_ref().draw(draw_handle, ev_mgr);
    }
}

impl<M> WidgetCore for Box<dyn Handler<Msg = M>> {
    fn core_data(&self) -> &CoreData {
        self.as_ref().core_data()
    }
    fn core_data_mut(&mut self) -> &mut CoreData {
        self.as_mut().core_data_mut()
    }

    fn as_widget(&self) -> &dyn Widget {
        self.as_ref().as_widget()
    }
    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self.as_mut().as_widget_mut()
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }
    fn get(&self, index: usize) -> Option<&dyn Widget> {
        self.as_ref().get(index)
    }
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
        self.as_mut().get_mut(index)
    }

    fn walk(&self, f: &mut dyn FnMut(&dyn Widget)) {
        self.as_ref().walk(f);
    }
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
        self.as_mut().walk_mut(f);
    }
}

impl<M> Clone for Box<dyn Handler<Msg = M>> {
    fn clone(&self) -> Self {
        #[cfg(feature = "nightly")]
        unsafe {
            let mut x = Box::new_uninit();
            self.clone_to(x.as_mut_ptr());
            x.assume_init()
        }

        // Run-time failure is not ideal — but we would hit compile-issues which
        // don't necessarily correspond to actual usage otherwise due to
        // `derive(Clone)` on any widget produced by `make_widget!`.
        #[cfg(not(feature = "nightly"))]
        panic!("Clone for Box<dyn Widget> only supported on nightly");
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
        let activable = widget.activation_via_press();
        match event {
            Event::Action(action) => widget.handle_action(tk, action),
            Event::Identify => Response::Identify(widget.id()),
            Event::PressStart { source, coord } if activable && source.is_primary() => {
                tk.update_data(&mut |data| {
                    data.request_press_grab(source, widget.as_widget(), coord)
                });
                Response::None
            }
            Event::PressMove { .. } if activable => {
                // We don't need these events, but they should not be considered *unhandled*
                Response::None
            }
            Event::PressEnd {
                start_id, end_id, ..
            } if activable && start_id == Some(widget.id()) && start_id == end_id => {
                widget.handle_action(tk, Action::Activate)
            }
            ev @ _ => Response::Unhandled(ev),
        }
    }
}
