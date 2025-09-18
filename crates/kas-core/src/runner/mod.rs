// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Runner, platforms and backends

mod common;
mod event_loop;
mod runner;
mod shared;
mod window;

use crate::messages::Erased;
use crate::window::{BoxedWindow, PopupDescriptor, WindowId};
use event_loop::Loop;
pub(crate) use shared::RunnerT;
use shared::Shared;
use std::fmt::Debug;
pub use window::Window;
pub(crate) use window::WindowDataErased;

pub use common::{Error, Platform, Result};
pub use runner::{ClosedError, PreLaunchState, Proxy};

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub use common::{GraphicsInstance, WindowSurface};

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub extern crate raw_window_handle;

/// A type-erased message stack
///
/// This is a stack over [`Erased`], with some downcasting methods.
/// It is a component of [`EventCx`](crate::event::EventCx) and usually only
/// used through that, thus the interface here is incomplete.
#[must_use]
#[derive(Debug, Default)]
pub struct MessageStack {
    base: usize,
    count: usize,
    stack: Vec<Erased>,
}

impl MessageStack {
    /// Construct an empty stack
    #[inline]
    pub fn new() -> Self {
        MessageStack::default()
    }

    /// Set the "stack base" to the current length
    ///
    /// Any messages on the stack before this method is called cannot be removed
    /// until the base has been reset. This allows multiple widget tree
    /// traversals with a single stack.
    #[inline]
    pub(crate) fn set_base(&mut self) {
        self.base = self.stack.len();
    }

    /// Get the current operation count
    ///
    /// This is incremented every time the message stack is changed.
    #[inline]
    pub(crate) fn get_op_count(&self) -> usize {
        self.count
    }

    /// Reset the base; return true if messages are available after reset
    #[inline]
    pub(crate) fn reset_and_has_any(&mut self) -> bool {
        self.base = 0;
        !self.stack.is_empty()
    }

    /// True if the stack has messages available
    #[inline]
    pub fn has_any(&self) -> bool {
        self.stack.len() > self.base
    }

    /// Push a type-erased message to the stack
    #[inline]
    pub(crate) fn push_erased(&mut self, msg: Erased) {
        self.count = self.count.wrapping_add(1);
        self.stack.push(msg);
    }

    /// Pop a type-erased message from the stack, if non-empty
    #[inline]
    pub fn pop_erased(&mut self) -> Option<Erased> {
        self.count = self.count.wrapping_add(1);
        self.stack.pop()
    }

    /// Try popping the last message from the stack with the given type
    pub fn try_pop<M: Debug + 'static>(&mut self) -> Option<M> {
        if self.has_any() && self.stack.last().map(|m| m.is::<M>()).unwrap_or(false) {
            self.count = self.count.wrapping_add(1);
            self.stack.pop().unwrap().downcast::<M>().ok().map(|m| *m)
        } else {
            None
        }
    }

    /// Try observing the last message on the stack without popping
    pub fn try_peek<M: Debug + 'static>(&self) -> Option<&M> {
        if self.has_any() {
            self.stack.last().and_then(|m| m.downcast_ref::<M>())
        } else {
            None
        }
    }

    /// Debug the last message on the stack, if any
    pub fn peek_debug(&self) -> Option<&dyn Debug> {
        self.stack.last().map(Erased::debug)
    }
}

impl Drop for MessageStack {
    fn drop(&mut self) {
        for msg in self.stack.drain(..) {
            if msg.is::<crate::event::components::KineticStart>() {
                // We can safely ignore this message
                continue;
            }

            log::warn!(target: "kas_core::erased", "unhandled: {msg:?}");
        }
    }
}

/// Application state
///
/// Kas allows application state to be stored both in the  widget tree (in
/// `Adapt` nodes and user-defined widgets) and by the application root (shared
/// across windows). This trait must be implemented by the latter.
///
/// When no top-level data is required, use `()` which implements this trait.
///
/// TODO: should we pass some type of interface to the runner to these methods?
/// We could pass a `&mut dyn RunnerT` easily, but that trait is not public.
pub trait AppData: 'static {
    /// Handle messages
    ///
    /// This is the last message handler: it is called when, after traversing
    /// the widget tree (see [kas::event] module doc), a message is left on the
    /// stack. Unhandled messages will result in warnings in the log.
    fn handle_messages(&mut self, messages: &mut MessageStack);

    /// Application is being suspended
    ///
    /// The application should ensure any important state is saved.
    ///
    /// This method is called when the application has been suspended or is
    /// about to exit (on Android/iOS/Web platforms, the application may resume
    /// after this method is called; on other platforms this probably indicates
    /// imminent closure). Widget state may still exist, but is not live
    /// (widgets will not process events or messages).
    fn suspended(&mut self) {}
}

impl AppData for () {
    fn handle_messages(&mut self, _: &mut MessageStack) {}
    fn suspended(&mut self) {}
}

enum Pending<A: AppData> {
    AddPopup(WindowId, WindowId, PopupDescriptor),
    RepositionPopup(WindowId, PopupDescriptor),
    AddWindow(WindowId, BoxedWindow<A>),
    CloseWindow(WindowId),
    Action(kas::Action),
    Exit,
}

#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(WindowId),
    Message(kas::messages::SendErased),
    WakeAsync,
    #[cfg(feature = "accesskit")]
    AccessKit(winit::window::WindowId, accesskit_winit::WindowEvent),
}

#[cfg(feature = "accesskit")]
impl From<accesskit_winit::Event> for ProxyAction {
    fn from(event: accesskit_winit::Event) -> Self {
        ProxyAction::AccessKit(event.window_id, event.window_event)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size_of_pending() {
        assert_eq!(std::mem::size_of::<Pending<()>>(), 40);
    }
}
