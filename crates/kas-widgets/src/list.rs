// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with run-time adjustable contents

use kas::dir::{Down, Right};
use kas::{event, layout, prelude::*};
use std::ops::{Index, IndexMut};

/// Support for optionally-indexed messages
pub trait FromIndexed<T> {
    fn from_indexed(i: usize, t: T) -> Self;
}
impl<T> FromIndexed<T> for T {
    #[inline]
    fn from_indexed(_: usize, t: T) -> Self {
        t
    }
}
impl<T> FromIndexed<T> for (usize, T) {
    #[inline]
    fn from_indexed(i: usize, t: T) -> Self {
        (i, t)
    }
}
impl<T> FromIndexed<T> for (u32, T) {
    #[inline]
    fn from_indexed(i: usize, t: T) -> Self {
        (i.cast(), t)
    }
}
impl<T> FromIndexed<T> for (u64, T) {
    #[inline]
    fn from_indexed(i: usize, t: T) -> Self {
        (i.cast(), t)
    }
}

/// A generic row widget
///
/// See documentation of [`List`] type. See also the [`row`](crate::row) macro.
pub type Row<W> = List<Right, W>;

/// A generic column widget
///
/// See documentation of [`List`] type. See also the [`column`](crate::column) macro.
pub type Column<W> = List<Down, W>;

/// A generic row widget
///
/// See documentation of [`IndexedList`] type.
pub type IndexedRow<W> = IndexedList<Right, W>;

/// A generic column widget
///
/// See documentation of [`IndexedList`] type.
pub type IndexedColumn<W> = IndexedList<Down, W>;

/// A row of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type BoxRow<M> = BoxList<Right, M>;

/// A column of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type BoxColumn<M> = BoxList<Down, M>;

/// A row/column of boxed widgets
///
/// This is parameterised over directionality and handler message type.
///
/// See documentation of [`List`] type.
pub type BoxList<D, M> = List<D, Box<dyn Widget<Msg = M>>>;

/// A generic row/column widget
///
/// This type is roughly [`Vec`] but for widgets. Generics:
///
/// -   `D:` [`Directional`] — fixed or run-time direction of layout
/// -   `W:` [`Widget`] — type of widget
///
/// The `List` widget forwards messages from children: `M = <W as Handler>::Msg`.
///
/// ## Alternatives
///
/// Some more specific type-defs are available:
///
/// -   [`Row`] fixes the direction to [`Right`]
/// -   [`Column`] fixes the direction to [`Down`]
/// -   [`row`](crate::row) and [`column`](crate::column) macros
/// -   [`BoxList`] is parameterised over the message type `M`, using boxed
///     widgets: `Box<dyn Widget<Msg = M>>`
/// -   [`BoxRow`] and [`BoxColumn`] are variants of [`BoxList`] with fixed direction
///
/// See also [`IndexedList`] and [`GenericList`] which allow other message types.
///
/// Where the entries are fixed, also consider custom [`Widget`] implementations.
///
/// ## Performance
///
/// Configuring and resizing elements is O(n) in the number of children.
/// Drawing and event handling is O(log n) in the number of children (assuming
/// only a small number are visible at any one time).
pub type List<D, W> = GenericList<D, W, <W as Handler>::Msg>;

/// A generic row/column widget
///
/// This type is roughly [`Vec`] but for widgets. Generics:
///
/// -   `D:` [`Directional`] — fixed or run-time direction of layout
/// -   `W:` [`Widget`] — type of widget
///
/// The `IndexedList` widget forwards messages from children together with the
/// child's index in the list: `(usize, M)` where `M = <W as Handler>::Msg`.
///
/// ## Alternatives
///
/// Some more specific type-defs are available:
///
/// -   [`IndexedRow`] fixes the direction to [`Right`]
/// -   [`IndexedColumn`] fixes the direction to [`Down`]
///
/// See also [`List`] and [`GenericList`] which allow other message types.
///
/// Where the entries are fixed, also consider custom [`Widget`] implementations.
///
/// ## Performance
///
/// Configuring and resizing elements is O(n) in the number of children.
/// Drawing and event handling is O(log n) in the number of children (assuming
/// only a small number are visible at any one time).
pub type IndexedList<D, W> = GenericList<D, W, (usize, <W as Handler>::Msg)>;

widget! {
    /// A generic row/column widget
    ///
    /// This type is roughly [`Vec`] but for widgets. Generics:
    ///
    /// -   `D:` [`Directional`] — fixed or run-time direction of layout
    /// -   `W:` [`Widget`] — type of widget
    /// -   `M` — the message type; restricted to `M:` [`FromIndexed`]`<M2>` where
    ///     `M2` is the child's message type; this is usually either `M2` or `(usize, M2)`
    ///
    /// ## Alternatives
    ///
    /// Some more specific type-defs are available:
    ///
    /// -   [`List`] fixes the message type to that of the child widget type `M`
    /// -   [`IndexedList`] fixes the message type to `(usize, M)`
    /// -   [`Row`], [`Column`], [`IndexedRow`], [`BoxList`], etc.
    ///
    /// Where the entries are fixed, also consider custom [`Widget`] implementations.
    ///
    /// ## Performance
    ///
    /// Configuring and resizing elements is O(n) in the number of children.
    /// Drawing and event handling is O(log n) in the number of children (assuming
    /// only a small number are visible at any one time).
    #[autoimpl(Clone where W: Clone)]
    #[autoimpl(Debug)]
    #[autoimpl(Default where D: Default)]
    #[handler(msg=M)]
    pub struct GenericList<
        D: Directional,
        W: Widget,
        M: FromIndexed<<W as Handler>::Msg> + 'static,
    > {
        first_id: WidgetId,
        #[widget_core]
        core: CoreData,
        widgets: Vec<W>,
        data: layout::DynRowStorage,
        direction: D,
        _pd: std::marker::PhantomData<M>,
    }

    impl WidgetChildren for Self {
        #[inline]
        fn first_id(&self) -> WidgetId {
            self.first_id
        }
        fn record_first_id(&mut self, id: WidgetId) {
            self.first_id = id;
        }
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn WidgetConfig> {
            self.widgets.get(index).map(|w| w.as_widget())
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
            self.widgets.get_mut(index).map(|w| w.as_widget_mut())
        }
    }

    impl Layout for Self {
        fn layout<'a>(&'a mut self) -> layout::Layout<'a> {
            let iter = self.widgets.iter_mut().map(|w| {
                layout::Layout::single(w.as_widget_mut(), AlignHints::NONE)
            });
            layout::Layout::list(iter, self.direction, &mut self.data, AlignHints::NONE)
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }

            let solver = layout::RowPositionSolver::new(self.direction);
            if let Some(child) = solver.find_child_mut(&mut self.widgets, coord) {
                return child.find_id(coord);
            }

            Some(self.id())
        }

        fn draw(&self, draw: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
            let disabled = disabled || self.is_disabled();
            let solver = layout::RowPositionSolver::new(self.direction);
            solver.for_children(&self.widgets, draw.get_clip_rect(), |w| {
                w.draw(draw, mgr, disabled)
            });
        }
    }

    impl event::SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if !self.is_disabled() {
                for (i, child) in self.widgets.iter_mut().enumerate() {
                    if id <= child.id() {
                        let r = child.send(mgr, id, event);
                        return match Response::try_from(r) {
                            Ok(r) => r,
                            Err(msg) => {
                                log::trace!(
                                    "Received by {} from {}: {:?}",
                                    self.id(),
                                    id,
                                    kas::util::TryFormat(&msg)
                                );
                                Response::Msg(FromIndexed::from_indexed(i, msg))
                            }
                        };
                    }
                }
            }

            Response::Unhandled
        }
    }

    impl Self where D: Default {
        /// Construct a new instance
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`Self::new_with_direction`].
        #[inline]
        pub fn new(widgets: Vec<W>) -> Self {
            Self::new_with_direction(D::default(), widgets)
        }
    }

    impl<W: Widget, M: FromIndexed<<W as Handler>::Msg> + 'static> GenericList<Direction, W, M> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> TkAction {
            self.direction = direction;
            // Note: most of the time SET_SIZE would be enough, but margins can be different
            TkAction::RESIZE
        }
    }

    impl Self {
        /// Construct a new instance with explicit direction
        #[inline]
        pub fn new_with_direction(direction: D, widgets: Vec<W>) -> Self {
            GenericList {
                first_id: Default::default(),
                core: Default::default(),
                widgets,
                data: Default::default(),
                direction,
                _pd: Default::default(),
            }
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// True if there are no child widgets
        pub fn is_empty(&self) -> bool {
            self.widgets.is_empty()
        }

        /// Returns the number of child widgets
        pub fn len(&self) -> usize {
            self.widgets.len()
        }

        /// Returns the number of elements the vector can hold without reallocating.
        pub fn capacity(&self) -> usize {
            self.widgets.capacity()
        }

        /// Reserves capacity for at least `additional` more elements to be inserted
        /// into the list. See documentation of [`Vec::reserve`].
        pub fn reserve(&mut self, additional: usize) {
            self.widgets.reserve(additional);
        }

        /// Remove all child widgets
        ///
        /// Triggers a [reconfigure action](Manager::send_action) if any widget is
        /// removed.
        pub fn clear(&mut self) -> TkAction {
            let action = match self.widgets.is_empty() {
                true => TkAction::empty(),
                false => TkAction::RECONFIGURE,
            };
            self.widgets.clear();
            action
        }

        /// Append a child widget
        ///
        /// Triggers a [reconfigure action](Manager::send_action).
        pub fn push(&mut self, widget: W) -> TkAction {
            self.widgets.push(widget);
            TkAction::RECONFIGURE
        }

        /// Remove the last child widget
        ///
        /// Returns `None` if there are no children. Otherwise, this
        /// triggers a reconfigure before the next draw operation.
        ///
        /// Triggers a [reconfigure action](Manager::send_action) if any widget is
        /// removed.
        pub fn pop(&mut self) -> (Option<W>, TkAction) {
            let action = match self.widgets.is_empty() {
                true => TkAction::empty(),
                false => TkAction::RECONFIGURE,
            };
            (self.widgets.pop(), action)
        }

        /// Inserts a child widget position `index`
        ///
        /// Panics if `index > len`.
        ///
        /// Triggers a [reconfigure action](Manager::send_action).
        pub fn insert(&mut self, index: usize, widget: W) -> TkAction {
            self.widgets.insert(index, widget);
            TkAction::RECONFIGURE
        }

        /// Removes the child widget at position `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// Triggers a [reconfigure action](Manager::send_action).
        pub fn remove(&mut self, index: usize) -> (W, TkAction) {
            let r = self.widgets.remove(index);
            (r, TkAction::RECONFIGURE)
        }

        /// Replace the child at `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// Triggers a [reconfigure action](Manager::send_action).
        // TODO: in theory it is possible to avoid a reconfigure where both widgets
        // have no children and have compatible size. Is this a good idea and can
        // we somehow test "has compatible size"?
        pub fn replace(&mut self, index: usize, mut widget: W) -> (W, TkAction) {
            std::mem::swap(&mut widget, &mut self.widgets[index]);
            (widget, TkAction::RECONFIGURE)
        }

        /// Append child widgets from an iterator
        ///
        /// Triggers a [reconfigure action](Manager::send_action) if any widgets
        /// are added.
        pub fn extend<T: IntoIterator<Item = W>>(&mut self, iter: T) -> TkAction {
            let len = self.widgets.len();
            self.widgets.extend(iter);
            match len == self.widgets.len() {
                true => TkAction::empty(),
                false => TkAction::RECONFIGURE,
            }
        }

        /// Resize, using the given closure to construct new widgets
        ///
        /// Triggers a [reconfigure action](Manager::send_action).
        pub fn resize_with<F: Fn(usize) -> W>(&mut self, len: usize, f: F) -> TkAction {
            let l0 = self.widgets.len();
            if l0 == len {
                return TkAction::empty();
            } else if l0 > len {
                self.widgets.truncate(len);
            } else {
                self.widgets.reserve(len);
                for i in l0..len {
                    self.widgets.push(f(i));
                }
            }
            TkAction::RECONFIGURE
        }

        /// Retain only widgets satisfying predicate `f`
        ///
        /// See documentation of [`Vec::retain`].
        ///
        /// Triggers a [reconfigure action](Manager::send_action) if any widgets
        /// are removed.
        pub fn retain<F: FnMut(&W) -> bool>(&mut self, f: F) -> TkAction {
            let len = self.widgets.len();
            self.widgets.retain(f);
            match len == self.widgets.len() {
                true => TkAction::empty(),
                false => TkAction::RECONFIGURE,
            }
        }

        /// Iterate over childern
        pub fn iter(&self) -> impl Iterator<Item = &W> {
            ListIter {
                list: &self.widgets,
            }
        }

        /// Mutably iterate over childern
        pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut W> {
            ListIterMut {
                list: &mut self.widgets,
            }
        }

        /// Get the index of the child which is an ancestor of `id`, if any
        pub fn find_child_index(&self, id: WidgetId) -> Option<usize> {
            if id >= self.first_id {
                for (i, child) in self.widgets.iter().enumerate() {
                    if id <= child.id() {
                        return Some(i);
                    }
                }
            }
            None
        }
    }

    impl Index<usize> for Self {
        type Output = W;

        fn index(&self, index: usize) -> &Self::Output {
            &self.widgets[index]
        }
    }

    impl IndexMut<usize> for Self {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            &mut self.widgets[index]
        }
    }
}

struct ListIter<'a, W: Widget> {
    list: &'a [W],
}
impl<'a, W: Widget> Iterator for ListIter<'a, W> {
    type Item = &'a W;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((first, rest)) = self.list.split_first() {
            self.list = rest;
            Some(first)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a, W: Widget> ExactSizeIterator for ListIter<'a, W> {
    fn len(&self) -> usize {
        self.list.len()
    }
}

struct ListIterMut<'a, W: Widget> {
    list: &'a mut [W],
}
impl<'a, W: Widget> Iterator for ListIterMut<'a, W> {
    type Item = &'a mut W;
    fn next(&mut self) -> Option<Self::Item> {
        let list = std::mem::take(&mut self.list);
        if let Some((first, rest)) = list.split_first_mut() {
            self.list = rest;
            Some(first)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a, W: Widget> ExactSizeIterator for ListIterMut<'a, W> {
    fn len(&self) -> usize {
        self.list.len()
    }
}
