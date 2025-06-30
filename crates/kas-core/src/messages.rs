// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Standard messages
//!
//! These are messages that may be sent via [`EventCx::push`](crate::event::EventCx::push).

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

    pub(crate) fn debug(&self) -> &dyn Debug {
        &self.0
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
