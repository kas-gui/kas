// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Erased type

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
