// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! # Standard messages
//!
//! These are pre-defined message types which may be sent to a widget via
//! [`EventCx::push`] or [`EventState::send`] in order to trigger some action.
//!
//! [`Erased`] is the type-erasure container allowing any type supporting
//! [`Any`] + [`Debug`] to be sent or placed on the message stack.

#[allow(unused)] use crate::event::{EventCx, EventState};
use std::any::Any;
use std::fmt::Debug;

use crate::event::PhysicalKey;

/// Synthetically trigger a "click" action
///
/// This message may be used to trigger a "click" action, for example to press a
/// button or toggle a check box state.
///
/// Payload: the key press which caused this message to be emitted, if any.
/// (This allows a visual state change to be bound to the key's release.)
#[derive(Copy, Clone, Debug)]
pub struct Activate(pub Option<PhysicalKey>);

/// Increment value by one step
#[derive(Copy, Clone, Debug)]
pub struct IncrementStep;

/// Decrement value by one step
#[derive(Copy, Clone, Debug)]
pub struct DecrementStep;

/// Set an input value from `f64`
///
/// This message may be used to set a numeric value to an input field.
#[derive(Copy, Clone, Debug)]
pub struct SetValueF64(pub f64);

/// Set an input value from a `String`
///
/// This message may be used to set a text value to an input field.
#[derive(Clone, Debug)]
pub struct SetValueText(pub String);

/// Replace selected text in an input value
///
/// This acts the same as typing or pasting the text: replace an existing
/// selection or insert at the cursor position.
#[derive(Clone, Debug)]
pub struct ReplaceSelectedText(pub String);

/// Set an index
#[derive(Clone, Debug)]
pub struct SetIndex(pub usize);

/// Request selection of the sender
///
/// This is only useful when pushed by a child widget or sent to a child widget
/// for usage by a parent container supporting selection. The recipient must use
/// [`EventCx::last_child`] to determine the selection target.
///
/// Example: a list supports selection; a child emits this to cause itself to be selected.
#[derive(Clone, Debug)]
pub struct Select;

trait AnyDebug: Any + Debug {}
impl<T: Any + Debug> AnyDebug for T {}

/// A type-erased message
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
