// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Standard messages
//!
//! These are messages that may be sent via [`EventCx::push`](crate::event::EventCx::push).

#[allow(unused)] use crate::Events;
use std::any::Any;
use std::fmt::Debug;

use crate::event::PhysicalKey;

/// Message: activate
///
/// Example: a button's label has a keyboard shortcut; this message is sent by the label to
/// trigger the button.
///
/// Payload: the key press which caused this message to be emitted, if any.
#[derive(Copy, Clone, Debug)]
pub struct Activate(pub Option<PhysicalKey>);

/// Message: select child
///
/// Example: a list supports selection; a child emits this to cause itself to be selected.
#[derive(Clone, Debug)]
pub struct Select;

trait AnyDebug: Any + Debug {}
impl<T: Any + Debug> AnyDebug for T {}

/// A type-erased value
///
/// This is vaguely a wrapper over `Box<dyn (Any + Debug)>`, except that Rust
/// doesn't (yet) support multi-trait objects.
#[derive(Debug)]
pub struct Erased(Box<dyn AnyDebug>);

impl Erased {
    /// Construct
    pub fn new<V: Any + Debug>(v: V) -> Self {
        Erased(Box::new(v))
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: 'static>(&self) -> bool {
        (&*self.0 as &dyn Any).is::<T>()
    }

    /// Attempt to downcast self to a concrete type.
    pub fn downcast<T: 'static>(self) -> Result<Box<T>, Box<dyn Any>> {
        (self.0 as Box<dyn Any>).downcast::<T>()
    }

    /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn’t.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        (&*self.0 as &dyn Any).downcast_ref::<T>()
    }
}

trait AnySendDebug: AnyDebug + Send {}
impl<T: Any + Send + Debug> AnySendDebug for T {}

/// Like Erased, but supporting Send
#[derive(Debug)]
pub(crate) struct SendErased(Box<dyn AnySendDebug>);

impl SendErased {
    /// Construct
    pub fn new<V: Any + Send + Debug>(v: V) -> Self {
        SendErased(Box::new(v))
    }

    /// Convert to [`Erased`]
    pub fn into_erased(self) -> Erased {
        Erased(self.0)
    }
}

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

    /// Try popping the last message from the stack with the given type
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_pop<M: Debug + 'static>(&mut self) -> Option<M> {
        if self.has_any() && self.stack.last().map(|m| m.is::<M>()).unwrap_or(false) {
            self.count = self.count.wrapping_add(1);
            self.stack.pop().unwrap().downcast::<M>().ok().map(|m| *m)
        } else {
            None
        }
    }

    /// Try observing the last message on the stack without popping
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_peek<M: Debug + 'static>(&self) -> Option<&M> {
        if self.has_any() {
            self.stack.last().and_then(|m| m.downcast_ref::<M>())
        } else {
            None
        }
    }

    /// Debug the last message on the stack, if any
    pub fn peek_debug(&self) -> Option<&dyn Debug> {
        self.stack.last().map(|m| &m.0 as &dyn Debug)
    }
}

impl Drop for MessageStack {
    fn drop(&mut self) {
        for msg in self.stack.drain(..) {
            if msg.is::<crate::event::components::GlideStart>() {
                // We can safely ignore this message
                continue;
            }

            log::warn!(target: "kas_core::erased", "unhandled: {msg:?}");
        }
    }
}
