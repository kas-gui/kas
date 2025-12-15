// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with wrapping

use crate::List;
use kas::Collection;
use kas::layout::{FlowSetter, FlowSolver, FlowStorage, RulesSetter, RulesSolver};
use kas::prelude::*;
use std::ops::{Index, IndexMut};

#[impl_self]
mod Flow {
    /// Rows or columns of content with line-splitting
    ///
    /// This widget is a variant of [`List`], arranging a linear [`Collection`]
    /// of children into multiple rows or columns with automatic splitting.
    /// Unlike [`Grid`](crate::Grid), items are not aligned across lines.
    ///
    /// When the collection uses [`Vec`], various methods to insert/remove
    /// elements are available.
    ///
    /// ## Layout details
    ///
    /// Currently only horizontal lines (rows) which wrap down to the next line
    /// are supported.
    ///
    /// Width requirements depend on the desired numbers of columns; see
    /// [`Self::set_num_columns`].
    ///
    /// Items within each line are stretched (if any has non-zero [`Stretch`]
    /// priority) in accordance with [`SizeRules::solve_widths`]. It is not
    /// currently possible to adjust this (except by tweaking the stretchiness
    /// of items).
    ///
    /// ## Performance
    ///
    /// Sizing, drawing and event handling are all `O(n)` where `n` is the number of children.
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
        layout: FlowStorage,
        secondary_is_reversed: bool,
        min_cols: i32,
        ideal_cols: i32,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let mut solver = FlowSolver::new(
                axis,
                self.list.direction.as_direction(),
                self.secondary_is_reversed,
                self.list.widgets.len(),
                &mut self.layout,
            );
            solver.set_num_columns(self.min_cols, self.ideal_cols);
            for n in 0..self.list.widgets.len() {
                if let Some(child) = self.list.widgets.get_mut_tile(n) {
                    solver.for_child(&mut self.layout, n, |axis| child.size_rules(cx, axis));
                }
            }
            solver.finish(&mut self.layout)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            self.list.core.set_rect(rect);
            let mut setter = FlowSetter::new(
                rect,
                self.list.direction.as_direction(),
                self.secondary_is_reversed,
                self.list.widgets.len(),
                &mut self.layout,
            );

            for n in 0..self.list.widgets.len() {
                if let Some(child) = self.list.widgets.get_mut_tile(n) {
                    child.set_rect(cx, setter.child_rect(&mut self.layout, n), hints);
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            // TODO(opt): use position solver as with List widget
            for child in self.list.widgets.iter_tile(..) {
                child.draw(draw.re());
            }
        }
    }

    impl Tile for Self {
        fn try_probe(&self, coord: Coord) -> Option<Id> {
            if !self.rect().contains(coord) {
                return None;
            }

            for child in self.list.widgets.iter_tile(..) {
                if let Some(id) = child.try_probe(coord) {
                    return Some(id);
                }
            }

            Some(self.id())
        }
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a new instance with default-constructed direction
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. The wrap direction is down or right.
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
        ///
        /// Lines flow from right-to-left, wrapping down.
        #[inline]
        pub fn left(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> Flow<C, kas::dir::Right> {
        /// Construct a new instance with fixed direction
        ///
        /// Lines flow from left-to-right, wrapping down.
        #[inline]
        pub fn right(widgets: C) -> Self {
            Self::new(widgets)
        }
    }

    impl Self {
        /// Construct a new instance with explicit direction
        #[inline]
        pub fn new_dir(widgets: C, direction: D) -> Self {
            assert!(
                direction.is_horizontal(),
                "column flow is not (yet) supported"
            );
            Flow {
                list: List::new_dir(widgets, direction),
                layout: Default::default(),
                secondary_is_reversed: false,
                min_cols: 1,
                ideal_cols: 3,
            }
        }

        /// Set the (minimum, ideal) numbers of columns
        ///
        /// This affects the final [`SizeRules`] for the horizontal axis.
        ///
        /// By default, the values `1, 3` are used.
        #[inline]
        pub fn set_num_columns(&mut self, min: i32, ideal: i32) {
            self.min_cols = min;
            self.ideal_cols = ideal;
        }

        /// Set the (minimum, ideal) numbers of columns (inline)
        ///
        /// This affects the final [`SizeRules`] for the horizontal axis.
        ///
        /// By default, the values `1, 3` are used.
        #[inline]
        pub fn with_num_columns(mut self, min: i32, ideal: i32) -> Self {
            self.set_num_columns(min, ideal);
            self
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
