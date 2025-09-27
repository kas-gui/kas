// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with run-time adjustable contents

use kas::Collection;
use kas::dir::{Down, Right};
use kas::layout::{
    DynRowStorage, RowPositionSolver, RowSetter, RowSolver, RowStorage, RulesSetter, RulesSolver,
};
use kas::prelude::*;
use std::collections::hash_map::{Entry, HashMap};
use std::ops::{Index, IndexMut};

/// Make a [`Row`] widget
///
/// # Syntax
///
/// > _Collection_ :\
/// > &nbsp;&nbsp; `row!` `[` _Items_<sup>\?</sup> `]`
/// >
/// > _Items_ :\
/// > &nbsp;&nbsp; (_Item_ `,`)<sup>\*</sup> _Item_ `,`<sup>\?</sup>
///
/// ## Stand-alone usage
///
/// When used as a stand-alone macro, `row! [/* ... */]` is just syntactic sugar
/// for `Row::new(kas::collection! [/* ... */])`.
///
/// In this case, _Item_ may be:
///
/// -   A string literal (interpreted as a label widget), optionally followed by
///     an [`align`] or [`pack`] method call
/// -   An expression yielding an object implementing `Widget<Data = _A>`
///
/// In case all _Item_ instances are a string literal, the data type of the
/// `row!` widget will be `()`; otherwise the data type of the widget is `_A`
/// where `_A` is a generic type parameter of the widget.
///
/// ## Usage within widget layout syntax
///
/// When called within [widget layout syntax], `row!` may be evaluated as a
/// recursive macro and the result does not have a specified type, except that
/// methods [`map_any`], [`align`] and [`pack`] are supported via emulation.
///
/// In this case, _Item_ is evaluated using [widget layout syntax]. This is
/// broadly similar to the above with a couple of exceptions:
///
/// -   Supported layout macros do not need to be imported to the module scope
/// -   An _Item_ may be a `#[widget]` field of the widget
///
/// # Example
///
/// ```
/// let my_widget = kas_widgets::row!["one", "two"];
/// ```
///
/// [widget layout syntax]: macro@kas::layout
/// [`map_any`]: crate::AdaptWidgetAny::map_any
/// [`align`]: crate::AdaptWidget::align
/// [`pack`]: crate::AdaptWidget::pack
#[macro_export]
macro_rules! row {
    ( $( $ee:expr ),* ) => {
        $crate::Row::new( ::kas::collection! [ $( $ee ),* ] )
    };
    ( $( $ee:expr ),+ , ) => {
        $crate::Row::new( ::kas::collection! [ $( $ee ),+ ] )
    };
}

/// Make a [`Column`] widget
///
/// # Syntax
///
/// > _Collection_ :\
/// > &nbsp;&nbsp; `column!` `[` _Items_<sup>\?</sup> `]`
/// >
/// > _Items_ :\
/// > &nbsp;&nbsp; (_Item_ `,`)<sup>\*</sup> _Item_ `,`<sup>\?</sup>
///
/// ## Stand-alone usage
///
/// When used as a stand-alone macro, `column! [/* ... */]` is just syntactic sugar
/// for `Column::new(kas::collection! [/* ... */])`.
///
/// In this case, _Item_ may be:
///
/// -   A string literal (interpreted as a label widget), optionally followed by
///     an [`align`] or [`pack`] method call
/// -   An expression yielding an object implementing `Widget<Data = _A>`
///
/// In case all _Item_ instances are a string literal, the data type of the
/// `column!` widget will be `()`; otherwise the data type of the widget is `_A`
/// where `_A` is a generic type parameter of the widget.
///
/// ## Usage within widget layout syntax
///
/// When called within [widget layout syntax], `column!` may be evaluated as a
/// recursive macro and the result does not have a specified type, except that
/// methods [`map_any`], [`align`] and [`pack`] are supported via emulation.
///
/// In this case, _Item_ is evaluated using [widget layout syntax]. This is
/// broadly similar to the above with a couple of exceptions:
///
/// -   Supported layout macros do not need to be imported to the module scope
/// -   An _Item_ may be a `#[widget]` field of the widget
///
/// # Example
///
/// ```
/// let my_widget = kas_widgets::column! [
///     "one",
///     "two",
/// ];
/// ```
///
/// [widget layout syntax]: macro@kas::layout
/// [`map_any`]: crate::AdaptWidgetAny::map_any
/// [`align`]: crate::AdaptWidget::align
/// [`pack`]: crate::AdaptWidget::pack
#[macro_export]
macro_rules! column {
    ( $( $ee:expr ),* ) => {
        $crate::Column::new( ::kas::collection! [ $( $ee ),* ] )
    };
    ( $( $ee:expr ),+ , ) => {
        $crate::Column::new( ::kas::collection! [ $( $ee ),+ ] )
    };
}

/// Make a [`List`] widget
///
/// # Syntax
///
/// > _Collection_ :\
/// > &nbsp;&nbsp; `list!` `[` _Items_<sup>\?</sup> `]`
/// >
/// > _Items_ :\
/// > &nbsp;&nbsp; (_Item_ `,`)<sup>\*</sup> _Item_ `,`<sup>\?</sup>
///
/// ## Stand-alone usage
///
/// When used as a stand-alone macro, `list! [/* ... */]` is just syntactic sugar
/// for `List::new(kas::collection! [/* ... */])`.
///
/// In this case, _Item_ may be:
///
/// -   A string literal (interpreted as a label widget), optionally followed by
///     an [`align`] or [`pack`] method call
/// -   An expression yielding an object implementing `Widget<Data = _A>`
///
/// In case all _Item_ instances are a string literal, the data type of the
/// `list!` widget will be `()`; otherwise the data type of the widget is `_A`
/// where `_A` is a generic type parameter of the widget.
///
/// ## Usage within widget layout syntax
///
/// When called within [widget layout syntax], `list!` may be evaluated as a
/// recursive macro and the result does not have a specified type, except that
/// methods [`map_any`], [`align`], [`pack`] and [`with_direction`] are
/// supported via emulation. In this case, calling [`with_direction`] is
/// required. Note that the argument passed to [`with_direction`] is expanded
/// at the use site, so for example `.with_direction(self.dir)` will read
/// `self.dir` whenever layout is computed.
///
/// In this case, _Item_ is evaluated using [widget layout syntax]. This is
/// broadly similar to the above with a couple of exceptions:
///
/// -   Supported layout macros do not need to be imported to the module scope
/// -   An _Item_ may be a `#[widget]` field of the widget
///
/// # Example
///
/// ```
/// let my_widget = kas_widgets::list! ["one", "two"]
///     .with_direction(kas::dir::Left);
/// ```
///
/// [widget layout syntax]: macro@kas::layout
/// [`map_any`]: crate::AdaptWidgetAny::map_any
/// [`align`]: crate::AdaptWidget::align
/// [`pack`]: crate::AdaptWidget::pack
/// [`with_direction`]: List::with_direction
#[macro_export]
macro_rules! list {
    ( $( $ee:expr ),* ) => {
        $crate::List::new( ::kas::collection! [ $( $ee ),* ] )
    };
    ( $( $ee:expr ),+ , ) => {
        $crate::List::new( ::kas::collection! [ $( $ee ),+ ] )
    };
}

/// A generic row widget
///
/// See documentation of [`List`] type.
pub type Row<C> = List<C, Right>;

/// A generic column widget
///
/// See documentation of [`List`] type.
pub type Column<C> = List<C, Down>;

#[impl_self]
mod List {
    /// A generic row/column widget
    ///
    /// A linear widget over a [`Collection`] of widgets.
    ///
    /// When the collection uses [`Vec`], various methods to insert/remove
    /// elements are available.
    ///
    /// The layout direction `D` may be compile-time fixed (e.g. [`Right`]) or
    /// run-time mutable ([`Direction`]); in the latter case
    /// [`set_direction`] is available.
    ///
    /// ## See also
    ///
    /// [`Row`] and [`Column`] are type-defs to `List` which fix the direction `D`.
    ///
    /// The macros [`row!`] and [`column!`] also create row/column
    /// layouts, but are not fully equivalent:
    ///
    /// -   `row!` and `column!` generate anonymous layout widgets (or objects).
    ///     These do not have a [`set_direction`] method or support adding or
    ///     removing elements.
    /// -   `row!` and `column!` generate layout objects which, when used within
    ///     a custom widget, may refer to that widget's fields.
    ///
    /// ## Performance
    ///
    /// Configuring and resizing elements is O(n) in the number of children.
    /// Drawing and event handling is O(log n) in the number of children (assuming
    /// only a small number are visible at any one time).
    ///
    /// ## Example
    ///
    /// ```
    /// use kas::collection;
    /// # use kas_widgets::{CheckBox, List};
    ///
    /// let list = List::right(collection![
    ///     "A checkbox",
    ///     CheckBox::new(|_, state: &bool| *state),
    /// ]);
    /// ```
    ///
    /// [`row!`]: crate::row
    /// [`column!`]: crate::column
    /// [`set_direction`]: List::set_direction
    #[autoimpl(Default where C: Default, D: Default)]
    #[widget]
    pub struct List<C: Collection, D: Directional> {
        core: widget_core!(),
        layout: DynRowStorage,
        #[collection]
        widgets: C,
        direction: D,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of Id to index
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let dim = (self.direction, self.widgets.len());
            let mut solver = RowSolver::new(axis, dim, &mut self.layout);
            for n in 0..self.widgets.len() {
                if let Some(child) = self.widgets.get_mut_tile(n) {
                    solver.for_child(&mut self.layout, n, |axis| {
                        child.size_rules(sizer.re(), axis)
                    });
                }
            }
            solver.finish(&mut self.layout)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            let dim = (self.direction, self.widgets.len());
            let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, &mut self.layout);

            for n in 0..self.widgets.len() {
                if let Some(child) = self.widgets.get_mut_tile(n) {
                    child.set_rect(cx, setter.child_rect(&mut self.layout, n), hints);
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let solver = RowPositionSolver::new(self.direction);
            solver.for_children(&self.widgets, draw.get_clip_rect(), |w| w.draw(draw.re()));
        }

        fn probe(&self, coord: Coord) -> Id {
            let solver = RowPositionSolver::new(self.direction);
            solver
                .find_child(&self.widgets, coord)
                .and_then(|child| child.try_probe(coord))
                .unwrap_or_else(|| self.id())
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::None
        }

        fn find_child_index(&self, id: &Id) -> Option<usize> {
            id.next_key_after(self.id_ref())
                .and_then(|k| self.id_map.get(&k).cloned())
        }
    }

    impl Events for Self {
        type Data = C::Data;

        /// Make a fresh id based on `self.next` then insert into `self.id_map`
        fn make_child_id(&mut self, index: usize) -> Id {
            if let Some(child) = self.widgets.get_tile(index) {
                // Use the widget's existing identifier, if valid
                if child.id_ref().is_valid() && self.id_ref().is_ancestor_of(child.id_ref()) {
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
            // All children will be re-configured which will rebuild id_map
            self.id_map.clear();
        }
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
        ///                     row.push(&mut cx.config_cx(), data, button);
        ///                 }
        ///                 Msg::Remove => {
        ///                     let _ = row.pop(&mut cx.config_cx());
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

    impl<C: Collection> List<C, kas::dir::Left> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn left(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> List<C, kas::dir::Right> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn right(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> List<C, kas::dir::Up> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn up(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> List<C, kas::dir::Down> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn down(widgets: C) -> Self {
            Self::new(widgets)
        }
    }

    impl<C: Collection, D: Directional + Eq> List<C, D> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, cx: &mut EventState, direction: D) {
            if direction == self.direction {
                return;
            }

            self.direction = direction;
            // Note: most of the time Action::SET_RECT would be enough, but margins can be different
            cx.resize(self);
        }
    }

    impl Self {
        /// Construct a new instance with explicit direction
        #[inline]
        pub fn new_dir(widgets: C, direction: D) -> Self {
            List {
                core: Default::default(),
                layout: Default::default(),
                widgets,
                direction,
                next: 0,
                id_map: Default::default(),
            }
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set the direction of contents (inline)
        #[inline]
        pub fn with_direction(mut self, direction: D) -> Self {
            self.direction = direction;
            self
        }

        /// Access layout storage
        ///
        /// The number of columns/rows is [`Self::len`].
        #[inline]
        pub fn layout_storage(&mut self) -> &mut (impl RowStorage + use<C, D>) {
            &mut self.layout
        }

        /// True if there are no child widgets
        pub fn is_empty(&self) -> bool {
            self.widgets.is_empty()
        }

        /// Returns the number of child widgets
        pub fn len(&self) -> usize {
            self.widgets.len()
        }
    }

    impl<W: Widget, D: Directional> List<Vec<W>, D> {
        /// Returns a reference to the child, if any
        pub fn get(&self, index: usize) -> Option<&W> {
            self.widgets.get(index)
        }

        /// Returns a mutable reference to the child, if any
        pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
            self.widgets.get_mut(index)
        }

        /// Remove all child widgets
        pub fn clear(&mut self) {
            self.widgets.clear();
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

            cx.resize(self);
            index
        }

        /// Remove the last child widget (if any) and return
        ///
        /// Triggers [`Action::RESIZE`].
        pub fn pop(&mut self, cx: &mut EventState) -> Option<W> {
            let result = self.widgets.pop();
            if let Some(w) = result.as_ref() {
                cx.resize(&self);

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
            cx.resize(self);
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

            cx.resize(&self);

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

            cx.resize(self);

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

            cx.resize(self);
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
                cx.resize(&self);
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
                cx.resize(self);
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

    impl<W: Widget, D: Directional> Index<usize> for List<Vec<W>, D> {
        type Output = W;

        fn index(&self, index: usize) -> &Self::Output {
            &self.widgets[index]
        }
    }

    impl<W: Widget, D: Directional> IndexMut<usize> for List<Vec<W>, D> {
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
