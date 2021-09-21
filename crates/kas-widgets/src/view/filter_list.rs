// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter-list view widget

use super::{driver, Driver, ListView, SelectionError, SelectionMode};
use crate::Scrollable;
use kas::event::ChildMsg;
use kas::prelude::*;
use kas::updatable::*;
use kas::updatable::{FilteredList, ListData, UpdatableAll};
use std::fmt::Debug;

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
#[derive(Debug, Widget)]
#[handler(handle=noauto, generics = <>)]
#[layout(single)]
#[widget(config=noauto)]
pub struct FilterListView<
    D: Directional,
    T: ListData + UpdatableAll<T::Key, V::Msg> + 'static,
    F: Filter<T::Item>,
    V: Driver<T::Item> = driver::Default,
> {
    #[widget_core]
    core: CoreData,
    #[widget]
    list: ListView<D, FilteredList<T, F>, V>,
}

impl<
        D: Directional + Default,
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item> + Default,
    > FilterListView<D, T, F, V>
{
    /// Construct a new instance
    ///
    /// This constructor is available where the direction is determined by the
    /// type: for `D: Directional + Default`. In other cases, use
    /// [`FilterListView::new_with_direction`].
    pub fn new(data: T, filter: F) -> Self {
        Self::new_with_dir_driver(D::default(), <V as Default>::default(), data, filter)
    }
}
impl<
        D: Directional,
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item> + Default,
    > FilterListView<D, T, F, V>
{
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, data: T, filter: F) -> Self {
        Self::new_with_dir_driver(direction, <V as Default>::default(), data, filter)
    }
}
impl<
        D: Directional + Default,
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item>,
    > FilterListView<D, T, F, V>
{
    /// Construct a new instance with explicit view
    pub fn new_with_driver(view: V, data: T, filter: F) -> Self {
        Self::new_with_dir_driver(D::default(), view, data, filter)
    }
}
impl<
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item> + Default,
    > FilterListView<Direction, T, F, V>
{
    /// Set the direction of contents
    pub fn set_direction(&mut self, direction: Direction) -> TkAction {
        self.list.set_direction(direction)
    }
}
impl<
        D: Directional,
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item>,
    > FilterListView<D, T, F, V>
{
    /// Construct a new instance with explicit direction and view
    pub fn new_with_dir_driver(direction: D, view: V, data: T, filter: F) -> Self {
        let data = FilteredList::new(data, filter);
        FilterListView {
            core: Default::default(),
            list: ListView::new_with_dir_driver(direction, view, data),
        }
    }

    // TODO: should this access pre-filter data?
    /// Access the stored data (pre-filter)
    pub fn data(&self) -> &T {
        &self.list.data().data
    }

    /// Mutably access the stored data (pre-filter)
    ///
    /// It may be necessary to use [`FilterListView::update_view`] to update the view of this data.
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.list.data_mut().data
    }

    /// Get a copy of the shared value at `key` (pre-filter)
    pub fn get_value(&self, key: &T::Key) -> Option<T::Item> {
        self.data().get_cloned(key)
    }

    /// Set shared data (pre-filter)
    ///
    /// This method updates the shared data, if supported (see
    /// [`ListData::update`]). Other widgets sharing this data are notified
    /// of the update, if data is changed.
    pub fn set_value(&self, mgr: &mut Manager, key: &T::Key, data: T::Item) {
        if let Some(handle) = self.data().update(key, data) {
            mgr.trigger_update(handle, 0);
        }
    }

    /// Update shared data (pre-filter)
    ///
    /// This is purely a convenience method over [`FilterListView::set_value`].
    /// It does nothing if no value is found at `key`.
    /// It notifies other widgets of updates to the shared data.
    pub fn update_value<G: Fn(T::Item) -> T::Item>(&self, mgr: &mut Manager, key: &T::Key, f: G) {
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
    pub fn clear_selected(&mut self) {
        self.list.clear_selected()
    }

    /// Directly select an item
    ///
    /// Returns `true` if selected, `false` if already selected.
    /// Fails if selection mode does not permit selection or if the key is
    /// invalid or filtered out.
    ///
    /// Does not send [`ChildMsg`] responses.
    pub fn select(&mut self, key: T::Key) -> Result<bool, SelectionError> {
        self.list.select(key)
    }

    /// Directly deselect an item
    ///
    /// Returns `true` if deselected, `false` if not previously selected.
    /// Also returns `false` on invalid and filtered-out keys.
    ///
    /// Does not send [`ChildMsg`] responses.
    pub fn deselect(&mut self, key: &T::Key) -> bool {
        self.list.deselect(key)
    }

    /// Manually trigger an update to handle changed data or filter
    pub fn update_view(&mut self, mgr: &mut Manager) {
        // TODO: update filter
        self.list.update_view(mgr)
    }

    /// Get the direction of contents
    pub fn direction(&self) -> Direction {
        self.list.direction()
    }

    /// Set the preferred number of items visible (inline)
    ///
    /// This affects the (ideal) size request and whether children are sized
    /// according to their ideal or minimum size but not the minimum size.
    pub fn with_num_visible(mut self, number: i32) -> Self {
        self.list = self.list.with_num_visible(number);
        self
    }
}

// TODO: support derive(Scrollable)?
impl<
        D: Directional,
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item>,
    > Scrollable for FilterListView<D, T, F, V>
{
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
    fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Offset) -> Offset {
        self.list.set_scroll_offset(mgr, offset)
    }
}

impl<
        D: Directional,
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item>,
    > WidgetConfig for FilterListView<D, T, F, V>
{
    fn configure(&mut self, mgr: &mut Manager) {
        // We must refresh the filtered list when the underlying list changes
        if let Some(handle) = self.list.data().data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
        // ... but we don't need to watch self.list.data().update_handle()
    }
}

impl<
        D: Directional,
        T: ListData + UpdatableAll<T::Key, V::Msg>,
        F: Filter<T::Item>,
        V: Driver<T::Item>,
    > Handler for FilterListView<D, T, F, V>
{
    type Msg = ChildMsg<T::Key, <V::Widget as Handler>::Msg>;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::HandleUpdate { .. } => {
                self.update_view(mgr);
                return Response::Update;
            }
            _ => Response::None,
        }
    }
}
