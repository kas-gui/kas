// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with run-time adjustable contents

use kas::dir::{Down, Right};
use kas::{layout, prelude::*};
use std::collections::hash_map::{Entry, HashMap};
use std::ops::{Index, IndexMut};

/// A generic row widget
///
/// See documentation of [`List`] type.
pub type Row<W> = List<W, Right>;

/// A generic column widget
///
/// See documentation of [`List`] type.
pub type Column<W> = List<W, Down>;

/// A row of boxed widgets
///
/// See documentation of [`List`] type.
pub type BoxRow<Data> = BoxList<Data, Right>;

/// A column of boxed widgets
///
/// See documentation of [`List`] type.
pub type BoxColumn<Data> = BoxList<Data, Down>;

/// A row/column of boxed widgets
///
/// This is parameterised over directionality.
///
/// See documentation of [`List`] type.
pub type BoxList<Data, D> = List<Box<dyn Widget<Data = Data>>, D>;

impl_scope! {
    /// A generic row/column widget
    ///
    /// This type is roughly [`Vec`] but for widgets. Generics:
    ///
    /// -   `W:` [`Widget`] — type of widget
    /// -   `D:` [`Directional`] — fixed or run-time direction of layout
    ///
    /// ## Alternatives
    ///
    /// Some more specific type-defs are available:
    ///
    /// -   [`Row`] and [`Column`] fix the direction `D`
    /// -   [`BoxList`] fixes the widget type to `Box<dyn Widget<Data = Data>>`
    /// -   [`BoxRow`] and [`BoxColumn`] fix both type parameters
    ///
    /// ## Performance
    ///
    /// Configuring and resizing elements is O(n) in the number of children.
    /// Drawing and event handling is O(log n) in the number of children (assuming
    /// only a small number are visible at any one time).
    ///
    /// # Messages
    ///
    /// If a handler is specified via [`Self::on_messages`] then this handler is
    /// called when a child pushes a message. This allows associating the
    /// child's index with a message.
    #[autoimpl(Default where D: Default)]
    #[widget {
        layout = slice! 'layout (self.direction, self.widgets);
    }]
    pub struct List<W: Widget, D: Directional> {
        core: widget_core!(),
        widgets: Vec<W>,
        direction: D,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of WidgetId to index
        on_messages: Option<Box<dyn Fn(&mut EventCx, usize)>>,
    }

    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Layout> {
            self.widgets.get(index).map(|w| w.as_layout())
        }

        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            id.next_key_after(self.id_ref())
                .and_then(|k| self.id_map.get(&k).cloned())
        }
    }

    impl Widget for Self {
        type Data = W::Data;

        fn for_child_node(
            &mut self,
            data: &W::Data,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get_mut(index) {
                closure(w.as_node(data));
            }
        }
    }

    impl Events for Self {
        /// Make a fresh id based on `self.next` then insert into `self.id_map`
        fn make_child_id(&mut self, index: usize) -> OwnedId {
            if let Some(child) = self.widgets.get(index) {
                // Use the widget's existing identifier, if any
                if child.id_ref().is_valid() {
                    if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                        if let Entry::Vacant(entry) = self.id_map.entry(key) {
                            entry.insert(index);
                            return child.id();
                        }
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

        fn configure(&mut self, _: &mut ConfigCx) {
            self.id_map.clear();
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(ref f) = self.on_messages {
                let index = cx.last_child().expect("message not sent from self");
                f(cx, index);
            }
        }
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a new instance
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`Self::new_dir`].
        pub fn new(widgets: impl Into<Vec<W>>) -> Self {
            Self::new_dir(widgets, D::default())
        }
    }

    impl<W: Widget> List<W, kas::dir::Left> {
        /// Construct a new instance
        pub fn left(widgets: impl Into<Vec<W>>) -> Self {
            Self::new(widgets)
        }
    }
    impl<W: Widget> List<W, kas::dir::Right> {
        /// Construct a new instance
        pub fn right(widgets: impl Into<Vec<W>>) -> Self {
            Self::new(widgets)
        }
    }
    impl<W: Widget> List<W, kas::dir::Up> {
        /// Construct a new instance
        pub fn up(widgets: impl Into<Vec<W>>) -> Self {
            Self::new(widgets)
        }
    }
    impl<W: Widget> List<W, kas::dir::Down> {
        /// Construct a new instance
        pub fn down(widgets: impl Into<Vec<W>>) -> Self {
            Self::new(widgets)
        }
    }

    impl<W: Widget> List<W, Direction> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> Action {
            if direction == self.direction {
                return Action::empty();
            }

            self.direction = direction;
            // Note: most of the time SET_RECT would be enough, but margins can be different
            Action::RESIZE
        }
    }

    impl Self {
        /// Construct a new instance with explicit direction
        #[inline]
        pub fn new_dir(widgets: impl Into<Vec<W>>, direction: D) -> Self {
            List {
                core: Default::default(),
                widgets: widgets.into(),
                direction,
                next: 0,
                id_map: Default::default(),
                on_messages: None,
            }
        }

        /// Assign a child message handler (inline style)
        ///
        /// This handler is called when a child pushes a message:
        /// `f(cx, index)`, where `index` is the child's index.
        #[inline]
        pub fn on_messages(mut self, f: impl Fn(&mut EventCx, usize) + 'static) -> Self {
            self.on_messages = Some(Box::new(f));
            self
        }

        /// Edit the list of children directly
        ///
        /// This may be used to edit children before window construction. It may
        /// also be used from a running UI, but in this case a full reconfigure
        /// of the window's widgets is required (triggered by the the return
        /// value, [`Action::RECONFIGURE`]).
        #[inline]
        pub fn edit<F: FnOnce(&mut Vec<W>)>(&mut self, f: F) -> Action {
            f(&mut self.widgets);
            Action::RECONFIGURE
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Access layout storage
        ///
        /// The number of columns/rows is [`Self.len`].
        #[inline]
        pub fn layout_storage(&mut self) -> &mut impl layout::RowStorage {
            &mut self.core.layout
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
        }

        /// Returns a reference to the child, if any
        pub fn get(&self, index: usize) -> Option<&W> {
            self.widgets.get(index)
        }

        /// Returns a mutable reference to the child, if any
        pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
            self.widgets.get_mut(index)
        }

        /// Append a child widget
        ///
        /// The new child is configured immediately. [`Action::RESIZE`] is
        /// triggered.
        ///
        /// Returns the new element's index.
        pub fn push(&mut self, cx: &mut ConfigCx, data: &W::Data, mut widget: W) -> usize {
            let index = self.widgets.len();
            let id = self.make_child_id(index);
            cx.configure(widget.as_node(data), id);
            self.widgets.push(widget);

            *cx |= Action::RESIZE;
            index
        }

        /// Remove the last child widget (if any) and return
        ///
        /// Triggers [`Action::RESIZE`].
        pub fn pop(&mut self, cx: &mut EventState) -> Option<W> {
            let result = self.widgets.pop();
            if let Some(w) = result.as_ref() {
                *cx |= Action::RESIZE;

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
        /// The new child is configured immediately. Triggers [`Action::RESIZE`].
        pub fn insert(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, mut widget: W) {
            for v in self.id_map.values_mut() {
                if *v >= index {
                    *v += 1;
                }
            }

            let id = self.make_child_id(index);
            cx.configure(widget.as_node(data), id);
            self.widgets.insert(index, widget);
            *cx |= Action::RESIZE;
        }

        /// Removes the child widget at position `index`
        ///
        /// Panics if `index` is out of bounds.
        ///
        /// Triggers [`Action::RESIZE`].
        pub fn remove(&mut self, cx: &mut EventState, index: usize) -> W {
            let w = self.widgets.remove(index);
            if w.id_ref().is_valid() {
                if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                    self.id_map.remove(&key);
                }
            }

            *cx |= Action::RESIZE;

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
        /// The new child is configured immediately. Triggers [`Action::RESIZE`].
        pub fn replace(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, mut w: W) -> W {
            let id = self.make_child_id(index);
            cx.configure(w.as_node(data), id);
            std::mem::swap(&mut w, &mut self.widgets[index]);

            if w.id_ref().is_valid() {
                if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                    self.id_map.remove(&key);
                }
            }

            *cx |= Action::RESIZE;

            w
        }

        /// Append child widgets from an iterator
        ///
        /// New children are configured immediately. Triggers [`Action::RESIZE`].
        pub fn extend<T>(&mut self, cx: &mut ConfigCx, data: &W::Data, iter: T)
        where
            T: IntoIterator<Item = W>,
        {
            let iter = iter.into_iter();
            if let Some(ub) = iter.size_hint().1 {
                self.widgets.reserve(ub);
            }
            for mut w in iter {
                let id = self.make_child_id(self.widgets.len());
                cx.configure(w.as_node(data), id);
                self.widgets.push(w);
            }

            *cx |= Action::RESIZE;
        }

        /// Resize, using the given closure to construct new widgets
        ///
        /// New children are configured immediately. Triggers [`Action::RESIZE`].
        pub fn resize_with<F>(&mut self, cx: &mut ConfigCx, data: &W::Data, len: usize, f: F)
        where
            F: Fn(usize) -> W,
        {
            let old_len = self.widgets.len();

            if len < old_len {
                *cx |= Action::RESIZE;
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
                    let id = self.make_child_id(index);
                    let mut w = f(index);
                    cx.configure(w.as_node(data), id);
                    self.widgets.push(w);
                }
                *cx |= Action::RESIZE;
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

impl<W: Widget, D: Directional + Default> FromIterator<W> for List<W, D> {
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = W>,
    {
        Self::new(iter.into_iter().collect::<Vec<W>>())
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
