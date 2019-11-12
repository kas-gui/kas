// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget traits

use std::fmt;

use crate::event::{Callback, EmptyMsg, Handler};
use crate::geom::{AxisInfo, Rect, Size, SizeRules};
use crate::toolkit::TkWindow;
use crate::{CoreData, WidgetId};

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

    /// Get the widget's numeric identifier
    #[inline]
    fn id(&self) -> WidgetId {
        self.core_data().id
    }

    /// Get the widget's region, relative to its parent.
    #[inline]
    fn rect(&self) -> Rect {
        self.core_data().rect
    }
}

/// Widget size and layout.
pub trait Layout: Core + fmt::Debug {
    /// Get size rules for the given axis.
    ///
    /// This method takes `&mut self` to allow local caching of child widget
    /// configuration for future `size_rules` and `set_rect` calls.
    ///
    /// If operating on one axis and the other is fixed, then the `other`
    /// parameter is used for the fixed dimension. Additionally, one may assume
    /// that `size_rules` has previously been called on the fixed axis with the
    /// current widget configuration.
    fn size_rules(&mut self, tk: &mut dyn TkWindow, axis: AxisInfo) -> SizeRules;

    /// Adjust to the given size.
    ///
    /// For many widgets this operation is trivial and the default
    /// implementation will suffice. For layout widgets (those with children),
    /// this operation is more complex.
    ///
    /// One may assume that `size_rules` has been called for each axis with the
    /// current widget configuration.
    #[inline]
    fn set_rect(&mut self, rect: Rect) {
        self.core_data_mut().rect = rect;
    }
}

/// Trait to describe the type needed by the [`Layout`] implementation.
///
/// To allow the `derive(Widget)` macro to implement [`Layout`], we use an
/// associated type to describe a data field of the following form:
/// ```none
/// #[layout_data] layout_data: <Self as kas::LayoutData>::Data,
/// ```
///
/// Ideally we would use an inherent associated type on the struct in question,
/// but until rust-lang#8995 is implemented that is not possible. We also cannot
/// place this associated type on the [`Layout`] trait itself, since then uses
/// of the trait would require parameterisation. Thus, this trait.
pub trait LayoutData {
    type Data: Clone + fmt::Debug + Default;
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
/// use kas::class::Class;
/// use kas::macros::Widget;
/// use kas::{Widget, CoreData};
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
    /// Erase type
    fn as_widget(&self) -> &dyn Widget;
    /// Erase type
    fn as_widget_mut(&mut self) -> &mut dyn Widget;

    /// Get the widget's classification
    ///
    /// This includes access to additional class-specific interfaces.
    fn class(&self) -> crate::class::Class;

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

    /// Find a child widget by identifier
    ///
    /// This requires that the widget tree has already been configured by
    /// [`crate::event::ManagerData::configure`].
    fn get_by_id(&self, id: WidgetId) -> Option<&dyn Widget> {
        if id == self.id() {
            return Some(self.as_widget());
        } else if id < self.id() {
            for i in 0..self.len() {
                if let Some(w) = self.get(i) {
                    if id <= w.id() {
                        return w.get_by_id(id);
                    }
                }
            }
        }
        None
    }

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk(&self, f: &mut dyn FnMut(&dyn Widget));

    /// Walk through all widgets, calling `f` once on each.
    ///
    /// This walk is iterative (nonconcurrent), depth-first, and always calls
    /// `f` on self *after* walking through all children.
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget));

    /// Debug tool: print the widget hierarchy
    fn print_hierarchy(&self, depth: usize) {
        println!(
            "{}W[{}]\t{:?}\t{:?}",
            "- ".repeat(depth),
            self.id(),
            self.class(),
            self.rect()
        );
        for i in 0..self.len() {
            self.get(i).unwrap().print_hierarchy(depth + 1);
        }
    }
}

/// A window is a drawable interactive region provided by windowing system.
// TODO: should this be a trait, instead of simply a struct? Should it be
// implemented by dialogs? Note that from the toolkit perspective, it seems a
// Window should be a Widget. So alternatives are (1) use a struct instead of a
// trait or (2) allow any Widget to derive Window (i.e. implement required
// functionality with macros instead of the generic code below).
pub trait Window: Widget + Handler<Msg = EmptyMsg> {
    /// Adjust the size of the window, repositioning widgets.
    fn resize(&mut self, tk: &mut dyn TkWindow, size: Size);

    /// Get a list of available callbacks.
    ///
    /// This returns a sequence of `(index, condition)` values. The toolkit
    /// should call `trigger_callback(index, tk)` whenever the condition is met.
    fn callbacks(&self) -> Vec<(usize, Callback)>;

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &mut dyn TkWindow);
}
