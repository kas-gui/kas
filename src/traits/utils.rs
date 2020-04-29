// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utility traits
//!
//! Arguably this stuff belongs in a different crate (even libstd).

/// Support trait for cloning boxed unsized objects
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub trait CloneTo {
    unsafe fn clone_to(&self, out: *mut Self);
}

impl<T: Clone + Sized> CloneTo for T {
    unsafe fn clone_to(&self, out: *mut Self) {
        let x = self.clone();
        std::ptr::copy(&x, out, 1);
        std::mem::forget(x);
    }
}

/// Provides a convenient `.boxed()` method on implementors
pub trait Boxed<T: ?Sized> {
    /// Boxing method
    fn boxed(self) -> Box<T>;
}
