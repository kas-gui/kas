// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with run-time adjustable contents

use kas::dir::{Down, Right};
use kas::{event, layout, prelude::*};
use std::collections::hash_map::{Entry, HashMap};
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
    #[widget { msg = M; }]
    pub struct GenericList<
        D: Directional,
        W: Widget,
        M: FromIndexed<<W as Handler>::Msg> + 'static,
    > {
        #[widget_core]
        core: CoreData,
        layout_store: layout::DynRowStorage,
        widgets: Vec<W>,
        direction: D,
        size_solved: bool,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of WidgetId to index
        _pd: std::marker::PhantomData<M>,
    }

    impl WidgetChildren for Self {
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

        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            id.next_key_after(self.id_ref()).and_then(|k| self.id_map.get(&k).cloned())
        }
    }

    impl WidgetConfig for Self {
        fn configure_recurse(&mut self, mgr: &mut SetRectMgr, id: WidgetId) {
            self.core_data_mut().id = id;
            self.id_map.clear();

            for index in 0..self.widgets.len() {
                let id = self.make_next_id(index);
                self.widgets[index].configure_recurse(mgr, id);
            }

            self.configure(mgr);
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            if self.size_solved {
                layout::Layout::slice(&mut self.widgets, self.direction, &mut self.layout_store)
            } else {
                // Draw without sizing all elements may cause a panic, so don't.
                Default::default()
            }
        }

        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            // Assumption: if size_rules is called, then set_rect will be too.
            self.size_solved = true;

            layout::Layout::slice(&mut self.widgets, self.direction, &mut self.layout_store)
                .size_rules(size_mgr, axis)
        }
    }

    impl event::SendEvent for Self {
        fn send(&mut self, mgr: &mut EventMgr, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if let Some(index) = self.find_child_index(&id) {
                if let Some(child) = self.widgets.get_mut(index) {
                    let r = child.send(mgr, id.clone(), event);
                    return match Response::try_from(r) {
                        Ok(r) => r,
                        Err(msg) => {
                            log::trace!(
                                "Received by {} from {}: {:?}",
                                self.id(),
                                id,
                                kas::util::TryFormat(&msg)
                            );
                            Response::Msg(FromIndexed::from_indexed(index, msg))
                        }
                    };
                }
            }

            Response::Unused
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
        // Assumption: index is a valid entry of self.widgets
        fn make_next_id(&mut self, index: usize) -> WidgetId {
            if let Some(child) = self.widgets.get(index) {
                // Use the widget's existing identifier, if any
                if child.id_ref().is_valid() {
                    if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                        self.id_map.insert(key, index);
                        return child.id();
                    }
                }
            }

            loop {
                let key = self.next;
                self.next += 1;
                if let Entry::Vacant(entry) = self.id_map.entry(key) {
                    entry.insert(index);
                    return self.id_ref().make_child(key);
                }
            }
        }

        /// Construct a new instance with explicit direction
        #[inline]
        pub fn new_with_direction(direction: D, widgets: Vec<W>) -> Self {
            GenericList {
                core: Default::default(),
                layout_store: Default::default(),
                widgets,
                direction,
                size_solved: false,
                next: 0,
                id_map: Default::default(),
                _pd: Default::default(),
            }
        }

        /// Edit the list of children directly
        ///
        /// This may be used to edit children before window construction. It may
        /// also be used from a running UI, but in this case a full reconfigure
        /// of the window's widgets is required (triggered by the the return
        /// value, [`TkAction::RECONFIGURE`]).
        #[inline]
        pub fn edit<F: FnOnce(&mut Vec<W>)>(&mut self, f: F) -> TkAction {
            f(&mut self.widgets);
            TkAction::RECONFIGURE
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

        /// Remove all child widgets
        pub fn clear(&mut self) {
            self.widgets.clear();
            self.size_solved = false;
        }

        /// Append a child widget
        ///
        /// The new child is configured immediately. [`TkAction::RESIZE`] is
        /// triggered.
        ///
        /// Returns the new element's index.
        pub fn push(&mut self, mgr: &mut SetRectMgr, widget: W) -> usize {
            let index = self.widgets.len();
            self.widgets.push(widget);
            let id = self.make_next_id(index);
            mgr.configure(id, &mut self.widgets[index]);
            self.size_solved = false;
            *mgr |= TkAction::RESIZE;
            index
        }

        /// Remove the last child widget (if any) and return
        ///
        /// Triggers [`TkAction::RESIZE`].
        pub fn pop(&mut self, mgr: &mut SetRectMgr) -> Option<W> {
            let result = self.widgets.pop();
            if let Some(w) = result.as_ref() {
                *mgr |= TkAction::RESIZE;

                if w.id_ref().is_valid() {
                    if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                        self.id_map.remove(&key);
                    }
                }
            }
            result
        }

        /// Inserts a child widget position `index`
        ///
        /// Panics if `index > len`.
        ///
        /// The new child is configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn insert(&mut self, mgr: &mut SetRectMgr, index: usize, widget: W) {
            for v in self.id_map.values_mut() {
                if *v >= index {
                    *v += 1;
                }
            }
            self.widgets.insert(index, widget);
            let id = self.make_next_id(index);
            mgr.configure(id, &mut self.widgets[index]);
            self.size_solved = false;
            *mgr |= TkAction::RESIZE;
        }

        /// Removes the child widget at position `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// Triggers [`TkAction::RESIZE`].
        pub fn remove(&mut self, mgr: &mut SetRectMgr, index: usize) -> W {
            let w = self.widgets.remove(index);
            if w.id_ref().is_valid() {
                if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                    self.id_map.remove(&key);
                }
            }

            *mgr |= TkAction::RESIZE;

            for v in self.id_map.values_mut() {
                if *v > index {
                    *v -= 1;
                }
            }
            w
        }

        /// Replace the child at `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// The new child is configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn replace(&mut self, mgr: &mut SetRectMgr, index: usize, mut w: W) -> W {
            std::mem::swap(&mut w, &mut self.widgets[index]);

            if w.id_ref().is_valid() {
                if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                    self.id_map.remove(&key);
                }
            }

            let id = self.make_next_id(index);
            mgr.configure(id, &mut self.widgets[index]);

            self.size_solved = false;
            *mgr |= TkAction::RESIZE;

            w
        }

        /// Append child widgets from an iterator
        ///
        /// New children are configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn extend<T: IntoIterator<Item = W>>(&mut self, mgr: &mut SetRectMgr, iter: T) {
            let old_len = self.widgets.len();
            self.widgets.extend(iter);
            for index in old_len..self.widgets.len() {
                let id = self.make_next_id(index);
                mgr.configure(id, &mut self.widgets[index]);
            }

            self.size_solved = false;
            *mgr |= TkAction::RESIZE;
        }

        /// Resize, using the given closure to construct new widgets
        ///
        /// New children are configured immediately. Triggers [`TkAction::RESIZE`].
        pub fn resize_with<F: Fn(usize) -> W>(&mut self, mgr: &mut SetRectMgr, len: usize, f: F) {
            let old_len = self.widgets.len();

            if len < old_len {
                *mgr |= TkAction::RESIZE;
                loop {
                    let w = self.widgets.pop().unwrap();
                    if w.id_ref().is_valid() {
                        if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                            self.id_map.remove(&key);
                        }
                    }
                    if len == self.widgets.len() {
                        return;
                    }
                }
            }

            if len > old_len {
                self.widgets.reserve(len - old_len);
                for index in old_len..len {
                    let id = self.make_next_id(index);
                    let mut widget = f(index);
                    mgr.configure(id, &mut widget);
                    self.widgets.push(widget);
                }
                self.size_solved = false;
                *mgr |= TkAction::RESIZE;
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
