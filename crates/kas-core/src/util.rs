// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utilities

use crate::WidgetId;
use std::fmt;

/// Helper to display widget identification (e.g. `MyWidget#01`)
///
/// Constructed by [`crate::WidgetExt::identify`].
pub struct IdentifyWidget(pub(crate) &'static str, pub(crate) WidgetId);
impl fmt::Display for IdentifyWidget {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}{}", self.0, self.1)
    }
}

/// Format for types supporting Debug
///
/// This requires the "spec" feature and nightly rustc to be useful.
pub struct TryFormat<'a, T: ?Sized>(pub &'a T);

#[cfg(not(feature = "spec"))]
impl<'a, T: ?Sized> fmt::Debug for TryFormat<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{}}}", std::any::type_name::<T>())
    }
}

#[cfg(feature = "spec")]
impl<'a, T: ?Sized> fmt::Debug for TryFormat<'a, T> {
    default fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{}}}", std::any::type_name::<T>())
    }
}

#[cfg(feature = "spec")]
impl<'a, T: fmt::Debug + ?Sized> fmt::Debug for TryFormat<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Generic implementation of [`crate::Widget::spatial_nav`]
pub fn spatial_nav(reverse: bool, from: Option<usize>, len: usize) -> Option<usize> {
    let last = len.wrapping_sub(1);
    if last == usize::MAX {
        return None;
    }

    if let Some(index) = from {
        match reverse {
            false if index < last => Some(index + 1),
            true if 0 < index => Some(index - 1),
            _ => None,
        }
    } else {
        match reverse {
            false => Some(0),
            true => Some(last),
        }
    }
}
