// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with wrapping

use crate::List;
use kas::Collection;
use kas::prelude::*;
use std::ops::{Index, IndexMut};

#[impl_self]
mod Flow {
    /// Rows or columns of content with line-splitting
    ///
    /// This widget is a variant of [`List`], arranging a linear [`Collection`]
    /// of children into multiple rows or columns with automatic splitting.
    /// See also [`Grid`](crate::Grid).
    ///
    /// When the collection uses [`Vec`], various methods to insert/remove
    /// elements are available.
    ///
    /// ## Example
    ///
    /// ```
    /// use kas::collection;
    /// # use kas_widgets::{CheckBox, Flow};
    ///
    /// let list = Flow::right(collection![
    ///     "A checkbox",
    ///     CheckBox::new(|_, state: &bool| *state),
    /// ]);
    /// ```
    ///
    /// [`row!`]: crate::row
    /// [`column!`]: crate::column
    /// [`set_direction`]: Flow::set_direction
    #[autoimpl(Default where C: Default, D: Default)]
    #[derive_widget]
    pub struct Flow<C: Collection, D: Directional> {
        #[widget]
        list: List<C, D>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a new instance with default-constructed direction
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`Self::new_dir`].
        ///
        /// # Examples
        ///
        /// Where widgets have the same type and the length is fixed, an array
        /// may be used:
        /// ```
        /// use kas_widgets::{Label, Row};
        /// let _ = Row::new([Label::new("left"), Label::new("right")]);
        /// ```
        ///
        /// To support run-time insertion/deletion, use [`Vec`]:
        /// ```
        /// use kas_widgets::{AdaptWidget, Button, Row};
        ///
        /// #[derive(Clone, Debug)]
        /// enum Msg {
        ///     Add,
        ///     Remove,
        /// }
        ///
        /// let _ = Row::new(vec![Button::label_msg("Add", Msg::Add)])
        ///     .on_messages(|cx, row, data| {
        ///         if let Some(msg) = cx.try_pop() {
        ///             match msg {
        ///                 Msg::Add => {
        ///                     let button = if row.len() % 2 == 0 {
        ///                         Button::label_msg("Add", Msg::Add)
        ///                     } else {
        ///                         Button::label_msg("Remove", Msg::Remove)
        ///                     };
        ///                     row.push(cx, data, button);
        ///                 }
        ///                 Msg::Remove => {
        ///                     let _ = row.pop(cx);
        ///                 }
        ///             }
        ///         }
        ///     });
        /// ```
        #[inline]
        pub fn new(widgets: C) -> Self {
            Self::new_dir(widgets, D::default())
        }
    }

    impl<C: Collection> Flow<C, kas::dir::Left> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn left(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> Flow<C, kas::dir::Right> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn right(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> Flow<C, kas::dir::Up> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn up(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> Flow<C, kas::dir::Down> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn down(widgets: C) -> Self {
            Self::new(widgets)
        }
    }

    impl Self {
        /// Construct a new instance with explicit direction
        #[inline]
        pub fn new_dir(widgets: C, direction: D) -> Self {
            Flow {
                list: List::new_dir(widgets, direction),
            }
        }

        /// True if there are no child widgets
        pub fn is_empty(&self) -> bool {
            self.list.is_empty()
        }

        /// Returns the number of child widgets
        pub fn len(&self) -> usize {
            self.list.len()
        }
    }

    impl<W: Widget, D: Directional> Flow<Vec<W>, D> {
        /// Returns a reference to the child, if any
        pub fn get(&self, index: usize) -> Option<&W> {
            self.list.get(index)
        }

        /// Returns a mutable reference to the child, if any
        pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
            self.list.get_mut(index)
        }

        /// Remove all child widgets
        pub fn clear(&mut self) {
            self.list.clear();
        }

        /// Append a child widget
        ///
        /// The new child is configured immediately. Triggers a resize.
        ///
        /// Returns the new element's index.
        pub fn push(&mut self, cx: &mut ConfigCx, data: &W::Data, widget: W) -> usize {
            self.list.push(cx, data, widget)
        }

        /// Remove the last child widget (if any) and return
        ///
        /// Triggers a resize.
        pub fn pop(&mut self, cx: &mut ConfigCx) -> Option<W> {
            self.list.pop(cx)
        }

        /// Inserts a child widget position `index`
        ///
        /// Panics if `index > len`.
        ///
        /// The new child is configured immediately. Triggers a resize.
        pub fn insert(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, widget: W) {
            self.list.insert(cx, data, index, widget);
        }

        /// Removes the child widget at position `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// Triggers a resize.
        pub fn remove(&mut self, cx: &mut ConfigCx, index: usize) -> W {
            self.list.remove(cx, index)
        }

        /// Removes all children at positions ≥ `len`
        ///
        /// Does nothing if `self.len() < len`.
        ///
        /// Triggers a resize.
        pub fn truncate(&mut self, cx: &mut ConfigCx, len: usize) {
            self.list.truncate(cx, len);
        }

        /// Replace the child at `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// The new child is configured immediately. Triggers a resize.
        pub fn replace(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, widget: W) -> W {
            self.list.replace(cx, data, index, widget)
        }

        /// Append child widgets from an iterator
        ///
        /// New children are configured immediately. Triggers a resize.
        pub fn extend<T>(&mut self, cx: &mut ConfigCx, data: &W::Data, iter: T)
        where
            T: IntoIterator<Item = W>,
        {
            self.list.extend(cx, data, iter);
        }

        /// Resize, using the given closure to construct new widgets
        ///
        /// New children are configured immediately. Triggers a resize.
        pub fn resize_with<F>(&mut self, cx: &mut ConfigCx, data: &W::Data, len: usize, f: F)
        where
            F: Fn(usize) -> W,
        {
            self.list.resize_with(cx, data, len, f);
        }

        /// Iterate over childern
        pub fn iter(&self) -> impl Iterator<Item = &W> {
            self.list.iter()
        }

        /// Mutably iterate over childern
        pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut W> {
            self.list.iter_mut()
        }
    }

    impl<W: Widget, D: Directional> Index<usize> for Flow<Vec<W>, D> {
        type Output = W;

        fn index(&self, index: usize) -> &Self::Output {
            self.list.index(index)
        }
    }

    impl<W: Widget, D: Directional> IndexMut<usize> for Flow<Vec<W>, D> {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            self.list.index_mut(index)
        }
    }
}
