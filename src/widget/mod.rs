// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widgets

#[cfg(not(feature = "layout"))]
mod layout_extern;

#[cfg(feature = "cassowary")]
#[macro_use]
mod layout_cw;

mod class;
mod data;

#[cfg(feature = "cassowary")]
pub use self::layout_cw::Layout;
#[cfg(not(feature = "layout"))]
pub use self::layout_extern::{ChildLayout, GridPos, Layout};

pub use self::class::Class;
pub use self::data::*;
use crate::toolkit::TkData;

/// Common widget behaviour
///
/// This is a base trait of [`Widget`] and should not need to be used directly.
/// It is implemented automatically by the `derive(Widget)` macro.
pub trait Core {
    /// Get the widget's number
    fn number(&self) -> u32;

    /// Set the widget's number
    ///
    /// This should only be called during widget enumeration. It will panic if
    /// the number has already been set (to anything other than 0).
    fn set_number(&mut self, number: u32);

    /// Get the toolkit data associated with this widget
    fn tkd(&self) -> TkData;

    /// Set the toolkit data associated with this widget
    ///
    /// This will panic if the toolkit data is not null.
    fn set_tkd(&mut self, tkd: TkData);

    /// Get the widget's region, relative to its parent.
    fn rect(&self) -> &Rect;

    /// Get mutable access to the widget's region
    fn rect_mut(&mut self) -> &mut Rect;
}

/// Common widget data
///
/// Widgets should normally implement `Core` by use of an embedded field
/// of this type (i.e. composition). See documentation of the [`Widget`] type.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    number: u32,
    tkd: TkData,
    rect: Rect,
}

impl Core for CoreData {
    #[inline]
    fn number(&self) -> u32 {
        self.number
    }

    #[inline]
    fn set_number(&mut self, number: u32) {
        if self.number != 0 {
            panic!("widget number has been set twice");
        }
        self.number = number;
    }

    #[inline]
    fn tkd(&self) -> TkData {
        self.tkd.clone()
    }

    #[inline]
    fn set_tkd(&mut self, tkd: TkData) {
        if !self.tkd.is_null() && !tkd.is_null() {
            panic!("widget's toolkit data has been set twice");
        }
        self.tkd = tkd;
    }

    #[inline]
    fn rect(&self) -> &Rect {
        &self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut Rect {
        &mut self.rect
    }
}

/// A widget encapsulates code for event handling and/or drawing some feature
/// of a sub-region of a window.
///
/// Widgets must usually also implement the [`Handler`] trait, which is separate
/// since it is generic.
///
/// This trait should *only* be implemented by using the `derive(Widget)` macro,
/// which can optionally also implement [`Handler`], as in the following example:
///
/// ```
/// use kas::{Class, Widget, CoreData};
/// use kas::macros::Widget;
///
/// #[widget(class = Class::Frame)]
/// #[handler]
/// #[derive(Clone, Debug, Widget)]
/// pub struct Frame<W: Widget> {
///     #[core] core: CoreData,
///     child: W,
/// }
/// ```
///
/// [`Handler`]: crate::event::Handler
pub trait Widget: Layout {
    /// Get the widget's classification
    ///
    /// This includes access to additional class-specific interfaces.
    fn class(&self) -> Class;

    /// Get the number of child widgets
    fn len(&self) -> usize;

    /// Get a reference to a child widget by index, or `None` if the index is
    /// out of bounds.
    ///
    /// For convenience, `Index<usize>` is implemented via this method.
    ///
    /// Required: `index < self.len()`.
    fn get(&self, index: usize) -> Option<&dyn Widget>;

    /// Mutable variant of get
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Widget>;

    #[doc(hidden)]
    /// This is only for use by toolkits.
    ///
    /// Set the number for self and each child. Returns own number + 1.
    fn enumerate(&mut self, mut n: u32) -> u32 {
        for i in 0..self.len() {
            self.get_mut(i).map(|w| n = w.enumerate(n));
        }
        self.set_number(n);
        n + 1
    }
}
