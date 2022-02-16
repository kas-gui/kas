// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter-list view widget

use super::{driver, Driver, ListView, SelectionError, SelectionMode};
use crate::Scrollable;
use kas::event::ChildMsg;
use kas::layout;
use kas::prelude::*;
use kas::updatable::filter::Filter;
use kas::updatable::{ListData, SingleData, Updatable};
use std::cell::RefCell;
use std::fmt::Debug;

/// Filter accessor over another accessor
///
/// This is an abstraction over a [`ListData`], applying a filter to items when
/// iterating and accessing.
///
/// When updating, the filter applies to the old value: if the old is included,
/// it is replaced by the new, otherwise no replacement occurs.
///
/// Note: the key and item types are the same as those in the underlying list,
/// thus one can also retrieve values from the underlying list directly.
///
/// Note: only `Rc<FilteredList<T, F>>` implements [`ListData`]; the [`Rc`]
/// wrapper is required!
///
/// Warning: this implementation is `O(n)` where `n = data.len()` and not well
/// optimised, thus is expected to be slow on large data lists.
#[derive(Clone, Debug)]
struct FilteredList<T: ListData, F: Filter<T::Item> + SingleData> {
    /// Direct access to unfiltered data
    data: T,
    /// Direct access to the filter
    filter: F,
    view: RefCell<Vec<T::Key>>,
}

impl<T: ListData, F: Filter<T::Item> + SingleData> FilteredList<T, F> {
    /// Construct and apply filter
    #[inline]
    fn new(data: T, filter: F) -> Self {
        let len = data.len();
        let view = RefCell::new(Vec::with_capacity(len));
        let s = FilteredList { data, filter, view };
        let _ = s.refresh();
        s
    }

    /// Refresh the view
    ///
    /// Re-applies the filter (`O(n)` where `n` is the number of data elements).
    /// Calling this directly may be useful in case the data is modified.
    fn refresh(&self) -> bool {
        let mut view = self.view.borrow_mut();
        view.clear();
        for key in self.data.iter_vec(usize::MAX) {
            if let Some(item) = self.data.get_cloned(&key) {
                if self.filter.matches(item) {
                    view.push(key);
                }
            }
        }
        true
    }
}

impl<K, M, T: ListData + Updatable<K, M> + 'static, F: Filter<T::Item> + SingleData> Updatable<K, M>
    for FilteredList<T, F>
{
    fn handle(&self, key: &K, msg: &M) -> bool {
        self.data.handle(key, msg)
    }
}

impl<T: ListData + 'static, F: Filter<T::Item> + SingleData> ListData for FilteredList<T, F> {
    type Key = T::Key;
    type Item = T::Item;

    fn version(&self) -> u64 {
        self.data.version() + self.filter.version()
    }

    fn len(&self) -> usize {
        self.view.borrow().len()
    }
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        self.data.make_id(parent, key)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        self.data.reconstruct_key(parent, child)
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        self.get_cloned(key).is_some()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        // Check the item against our filter (probably O(1)) instead of using
        // our filtered list (O(n) where n=self.len()).
        self.data
            .get_cloned(key)
            .filter(|item| self.filter.matches(item.clone()))
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> bool {
        // Filtering does not affect result, but does affect the view
        if self
            .data
            .get_cloned(key)
            .map(|item| !self.filter.matches(item))
            .unwrap_or(true)
        {
            // Not previously visible: no update occurs
            return false;
        }

        let new_visible = self.filter.matches(value.clone());
        let result = self.data.update(key, value);
        if result && !new_visible {
            // remove the updated item from our filtered list
            self.view.borrow_mut().retain(|item| item != key);
        }
        result
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
        let end = self.len().min(start + limit);
        self.view.borrow()[start..end].to_vec()
    }
}

widget! {
    /// Filter-list view widget
    ///
    /// This widget is a wrapper around [`ListView`] which applies a filter to the
    /// data list.
    ///
    /// Why is a data-filter a widget and not a pure-data item, you ask? The answer
    /// is that a filter-list must be updated when the filter or the data changes,
    /// and, since filtering a list is not especially cheap, the filtering must be
    /// cached and updated when required, not every time the list view asks for more
    /// data. Although it is possible to do this with a data-view, that requires
    /// machinery for recursive-updates on data-structures and/or a mechanism to
    /// test whether the underlying list-data changed. Implementing as a widget
    /// avoids this.
    // TODO: impl Clone
    #[derive(Debug)]
    #[handler(msg = ChildMsg<T::Key, <V::Widget as Handler>::Msg>)]
    pub struct FilterListView<
        D: Directional,
        T: ListData + Updatable<T::Key, V::Msg> + 'static,
        F: Filter<T::Item> + SingleData,
        V: Driver<T::Item> = driver::Default,
    > {
        #[widget_core]
        core: CoreData,
        #[widget]
        list: ListView<D, FilteredList<T, F>, V>,
        data_ver: u64,
    }

    impl Self where D: Default, V: Default {
        /// Construct a new instance
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`FilterListView::new_with_direction`].
        pub fn new(data: T, filter: F) -> Self {
            Self::new_with_dir_driver(D::default(), <V as Default>::default(), data, filter)
        }
    }
    impl Self where V: Default {
        /// Construct a new instance with explicit direction
        pub fn new_with_direction(direction: D, data: T, filter: F) -> Self {
            Self::new_with_dir_driver(direction, <V as Default>::default(), data, filter)
        }
    }
    impl Self where D: Default {
        /// Construct a new instance with explicit view
        pub fn new_with_driver(view: V, data: T, filter: F) -> Self {
            Self::new_with_dir_driver(D::default(), view, data, filter)
        }
    }
    impl<
            T: ListData + Updatable<T::Key, V::Msg>,
            F: Filter<T::Item> + SingleData,
            V: Driver<T::Item> + Default,
        > FilterListView<Direction, T, F, V>
    {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> TkAction {
            self.list.set_direction(direction)
        }
    }
    impl Self {
        /// Construct a new instance with explicit direction and view
        pub fn new_with_dir_driver(direction: D, view: V, data: T, filter: F) -> Self {
            let data = FilteredList::new(data, filter);
            FilterListView {
                core: Default::default(),
                list: ListView::new_with_dir_driver(direction, view, data),
                data_ver: 0,
            }
        }

        /// Access the stored data (pre-filter)
        pub fn unfiltered_data(&self) -> &T {
            &self.list.data().data
        }

        /// Mutably access the stored data (pre-filter)
        pub fn unfiltered_data_mut(&mut self) -> &mut T {
            &mut self.list.data_mut().data
        }

        /// Access the stored data (post-filter)
        pub fn data(&self) -> &T {
            &self.list.data().data
        }

        /// Mutably access the stored data (post-filter)
        pub fn data_mut(&mut self) -> &mut T {
            &mut self.list.data_mut().data
        }

        /// Check whether a key has data (post-filter)
        pub fn contains_key(&self, key: &T::Key) -> bool {
            self.data().contains_key(key)
        }

        /// Get a copy of the shared value at `key` (post-filter)
        pub fn get_value(&self, key: &T::Key) -> Option<T::Item> {
            self.data().get_cloned(key)
        }

        /// Set shared data (post-filter)
        ///
        /// This method updates the shared data, if supported (see
        /// [`ListData::update`]). Other widgets sharing this data are notified
        /// of the update, if data is changed.
        pub fn set_value(&self, mgr: &mut EventMgr, key: &T::Key, data: T::Item) {
            if self.data().update(key, data) {
                mgr.redraw_all_windows();
            }
        }

        /// Update shared data (post-filter)
        ///
        /// This is purely a convenience method over [`FilterListView::set_value`].
        /// It does nothing if no value is found at `key`.
        /// It notifies other widgets of updates to the shared data.
        pub fn update_value<G: Fn(T::Item) -> T::Item>(&self, mgr: &mut EventMgr, key: &T::Key, f: G) {
            if let Some(item) = self.get_value(key) {
                self.set_value(mgr, key, f(item));
            }
        }

        /// Get the current selection mode
        pub fn selection_mode(&self) -> SelectionMode {
            self.list.selection_mode()
        }
        /// Set the current selection mode
        pub fn set_selection_mode(&mut self, mode: SelectionMode) -> TkAction {
            self.list.set_selection_mode(mode)
        }
        /// Set the selection mode (inline)
        #[must_use]
        pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
            let _ = self.set_selection_mode(mode);
            self
        }

        /// Read the list of selected entries
        ///
        /// With mode [`SelectionMode::Single`] this may contain zero or one entry;
        /// use `selected_iter().next()` to extract only the first (optional) entry.
        pub fn selected_iter(&'_ self) -> impl Iterator<Item = &'_ T::Key> + '_ {
            self.list.selected_iter()
        }

        /// Check whether an entry is selected
        pub fn is_selected(&self, key: &T::Key) -> bool {
            self.list.is_selected(key)
        }

        /// Clear all selected items
        ///
        /// Does not send [`ChildMsg`] responses.
        pub fn clear_selected(&mut self) -> TkAction {
            self.list.clear_selected()
        }

        /// Directly select an item
        ///
        /// Returns `TkAction::REDRAW` if newly selected, `TkAction::empty()` if
        /// already selected. Fails if selection mode does not permit selection
        /// or if the key is invalid.
        ///
        /// Does not send [`ChildMsg`] responses.
        pub fn select(&mut self, key: T::Key) -> Result<TkAction, SelectionError> {
            self.list.select(key)
        }

        /// Directly deselect an item
        ///
        /// Returns `TkAction::REDRAW` if deselected, `TkAction::empty()` if not
        /// previously selected or if the key is invalid.
        ///
        /// Does not send [`ChildMsg`] responses.
        pub fn deselect(&mut self, key: &T::Key) -> TkAction {
            self.list.deselect(key)
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.list.direction()
        }

        /// Set the preferred number of items visible (inline)
        ///
        /// This affects the (ideal) size request and whether children are sized
        /// according to their ideal or minimum size but not the minimum size.
        #[must_use]
        pub fn with_num_visible(mut self, number: i32) -> Self {
            self.list = self.list.with_num_visible(number);
            self
        }
    }

    // TODO: support derive(Scrollable)?
    impl Scrollable for Self {
        #[inline]
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            self.list.scroll_axes(size)
        }

        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.list.max_scroll_offset()
        }

        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.list.scroll_offset()
        }

        #[inline]
        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            self.list.set_scroll_offset(mgr, offset)
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            layout::Layout::single(&mut self.list)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let data_ver = self.list.data().version();
            if data_ver > self.data_ver {
                self.list.data_mut().refresh();
                self.data_ver = data_ver;
            }

            let draw = draw.with_core(self.core_data());
            self.layout().draw(draw);
        }
    }
}
