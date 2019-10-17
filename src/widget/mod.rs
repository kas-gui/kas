// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widgets

#[macro_use]
mod layout;

mod class;
mod data;

pub use self::layout::*;

pub use self::class::Class;
pub use self::data::*;

/// Common widget data
///
/// This is a base trait of [`Widget`] and usually implemented by the
/// `derive(Widget)` macro.
pub trait Core {
    /// Get direct access to the [`CoreData`] providing property storage.
    fn core_data(&self) -> &CoreData;

    /// Get mutable access to the [`CoreData`] providing property storage.
    ///
    /// This should not normally be needed by user code.
    #[doc(hidden)]
    fn core_data_mut(&mut self) -> &mut CoreData;

    /// Get the widget's number
    #[inline]
    fn number(&self) -> u32 {
        self.core_data().number
    }

    /// Get the widget's region, relative to its parent.
    #[inline]
    fn rect(&self) -> Rect {
        self.core_data().rect
    }
}

/// Common widget data
///
/// All widgets should embed a `core: CoreData` field in order to implement the
/// [`Core`] macro.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    pub number: u32,
    pub rect: Rect,
}

impl Core for CoreData {
    #[inline]
    fn core_data(&self) -> &CoreData {
        self
    }

    #[inline]
    fn core_data_mut(&mut self) -> &mut CoreData {
        self
    }
}

/// A widget encapsulates code for event handling and/or drawing some feature
/// of a sub-region of a window.
///
/// Widgets must usually also implement the [`Handler`] trait, which is separate
/// since it is generic.
///
/// This trait should *only* be implemented by using the `derive(Widget)` macro.
/// This macro additionally implements the [`Core`] trait, and optionally also
/// the [`Layout`] and [`Handler`] traits. See documentation in the
/// [`kas::macros`] module.
///
/// ```
/// use kas::{Class, Widget, CoreData};
/// use kas::macros::Widget;
///
/// #[widget(class = Class::Frame, layout = single)]
/// #[derive(Clone, Debug, Widget)]
/// pub struct Frame<W: Widget> {
///     #[core] core: CoreData,
///     #[widget] child: W,
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
        self.core_data_mut().number = n;
        n + 1
    }

    /// Debug tool: print the widget hierarchy
    fn print_hierarchy(&self, depth: usize) {
        println!(
            "{}W[{}]\t{:?}\t{:?}",
            "- ".repeat(depth),
            self.number(),
            self.class(),
            self.rect()
        );
        for i in 0..self.len() {
            self.get(i).unwrap().print_hierarchy(depth + 1);
        }
    }
}
