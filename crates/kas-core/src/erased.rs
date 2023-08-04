// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Erased type

#![cfg_attr(not(winit), allow(unused))]

use crate::Action;
#[allow(unused)] use crate::Events;
use std::any::Any;
use std::fmt::Debug;

/// A type-erased value
///
/// This is vaguely a wrapper over `Box<dyn (Any + Debug)>`, except that Rust
/// doesn't (yet) support multi-trait objects.
pub struct Erased {
    // TODO: use trait_upcasting feature when stable: Box<dyn AnyDebug>
    // where trait AnyDebug: Any + Debug {}. This replaces the fmt field.
    any: Box<dyn Any>,
    #[cfg(debug_assertions)]
    fmt: String,
}

impl Erased {
    /// Construct
    pub fn new<V: Any + Debug>(v: V) -> Self {
        #[cfg(debug_assertions)]
        let fmt = format!("{}::{:?}", std::any::type_name::<V>(), &v);
        let any = Box::new(v);
        Erased {
            #[cfg(debug_assertions)]
            fmt,
            any,
        }
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: 'static>(&self) -> bool {
        self.any.is::<T>()
    }

    /// Attempt to downcast self to a concrete type.
    pub fn downcast<T: 'static>(self) -> Result<Box<T>, Box<dyn Any>> {
        self.any.downcast::<T>()
    }

    /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn’t.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.any.downcast_ref::<T>()
    }
}

/// Support debug formatting
///
/// Debug builds only. On release builds, a placeholder message is printed.
impl std::fmt::Debug for Erased {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        #[cfg(debug_assertions)]
        let r = f.write_str(&self.fmt);
        #[cfg(not(debug_assertions))]
        let r = f.write_str("[use debug build to see value]");
        r
    }
}

/// Like Erased, but supporting Send
pub(crate) struct SendErased {
    any: Box<dyn Any + Send>,
    #[cfg(debug_assertions)]
    fmt: String,
}

impl SendErased {
    /// Construct
    pub fn new<V: Any + Send + Debug>(v: V) -> Self {
        #[cfg(debug_assertions)]
        let fmt = format!("{}::{:?}", std::any::type_name::<V>(), &v);
        let any = Box::new(v);
        SendErased {
            #[cfg(debug_assertions)]
            fmt,
            any,
        }
    }

    /// Convert to [`Erased`]
    pub fn into_erased(self) -> Erased {
        Erased {
            any: self.any,
            #[cfg(debug_assertions)]
            fmt: self.fmt,
        }
    }
}

/// A type-erased message stack
///
/// This is a stack over [`Erased`], with some downcasting methods.
/// It is a component of [`EventCx`](crate::event::EventCx) and usually only
/// used through that, thus the interface here is incomplete.
#[must_use]
#[derive(Debug, Default)]
pub struct ErasedStack {
    base: usize,
    stack: Vec<Erased>,
}

impl ErasedStack {
    /// Construct an empty stack
    #[inline]
    pub fn new() -> Self {
        ErasedStack::default()
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
        self.stack.push(msg);
    }

    /// Try popping the last message from the stack with the given type
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_pop<M: Debug + 'static>(&mut self) -> Option<M> {
        if self.has_any() && self.stack.last().map(|m| m.is::<M>()).unwrap_or(false) {
            self.stack.pop().unwrap().downcast::<M>().ok().map(|m| *m)
        } else {
            None
        }
    }

    /// Try observing the last message on the stack without popping
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_observe<M: Debug + 'static>(&self) -> Option<&M> {
        if self.has_any() {
            self.stack.last().and_then(|m| m.downcast_ref::<M>())
        } else {
            None
        }
    }
}

impl Drop for ErasedStack {
    fn drop(&mut self) {
        for msg in self.stack.drain(..) {
            log::warn!(target: "kas_core::erased", "unhandled: {msg:?}");
        }
    }
}

/// Application state
///
/// Kas allows state to be stored in `Adapt` and user-defined widgets within
/// windows, but sometimes you want top-level application state too (especially
/// for data shared between windows). Such state implements this trait and is
/// passed to the shell/runner's constructor.
///
/// When no top-level data is required, use `()` which implements this trait.
pub trait AppData: 'static {
    /// Handle messages
    ///
    /// This is the last message handler: it is called when, after traversing
    /// the widget tree (see [kas::event] module doc), a message is left on the
    /// stack. Unhandled messages will result in warnings in the log.
    ///
    /// The method returns an [`Action`], usually either [`Action::empty`]
    /// (nothing to do) or [`Action::UPDATE`] (to update widgets).
    /// This action affects all windows.
    fn handle_messages(&mut self, messages: &mut ErasedStack) -> Action;
}

impl AppData for () {
    fn handle_messages(&mut self, _: &mut ErasedStack) -> Action {
        Action::empty()
    }
}
