// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A grid widget

use std::ops::{Index, IndexMut};

use kas::layout::{self, GridChildInfo, RulesSetter, RulesSolver};
use kas::{event, prelude::*};

/// A grid of boxed widgets
///
/// This is a parameterisation of [`Grid`]
/// This is parameterised over the handler message type.
///
/// See documentation of [`Grid`] type.
pub type BoxGrid<M> = Grid<Box<dyn Widget<Msg = M>>>;

/// A generic grid widget
///
/// Child widgets are displayed in a grid, according to each child's
/// [`GridChildInfo`]. This allows spans and overlapping widgets. The numbers
/// of rows and columns is determined automatically while the sizes of rows and
/// columns are determined based on their contents (including special handling
/// for spans, *mostly* with good results).
///
/// Note that all child widgets are stored in a list internally. The order of
/// widgets in that list does not affect display position, but does have a few
/// effects: (a) widgets may be accessed in this order via indexing, (b) widgets
/// are configured and drawn in this order, (c) navigating
/// through widgets with the Tab key currently uses the list order (though it
/// may be changed in the future to use display order).
///
/// Most operations (configuring, resizing and drawing) are O(n) in the number
/// of children. This type is generic over the type of child widgets.
///
/// For fixed layouts, an alternative is to construct a custom widget with
/// [`kas::macros::make_widget`] which can also perform event handling.
#[derive(Clone, Default, Debug, Widget)]
#[handler(send=noauto, msg=(usize, <W as event::Handler>::Msg))]
#[widget(children=noauto)]
pub struct Grid<W: Widget> {
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    widgets: Vec<(W, GridChildInfo)>,
    data: layout::DynGridStorage,
}

impl<W: Widget> WidgetChildren for Grid<W> {
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
        self.widgets.get(index).map(|w| w.0.as_widget())
    }
    #[inline]
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        self.widgets.get_mut(index).map(|w| w.0.as_widget_mut())
    }
}

impl<W: Widget> Layout for Grid<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let dim = self.dimensions();
        let mut solver = layout::GridSolver::<Vec<_>, Vec<_>, _>::new(axis, dim, &mut self.data);
        for child in self.widgets.iter_mut() {
            solver.for_child(&mut self.data, child.1, |axis| {
                child.0.size_rules(size_handle, axis)
            });
        }
        solver.finish(&mut self.data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let dim = self.dimensions();
        let mut setter =
            layout::GridSetter::<Vec<i32>, Vec<i32>, _>::new(rect, dim, align, &mut self.data);

        for child in self.widgets.iter_mut() {
            child
                .0
                .set_rect(mgr, setter.child_rect(&mut self.data, child.1), align);
        }
    }

    // TODO: we should probably implement spatial_nav (the same is true for
    // macro-generated grid widgets).
    // fn spatial_nav(&self, reverse: bool, from: Option<usize>) -> Option<usize> { .. }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        // TODO(opt): more efficient position solver (also for drawing)?
        for child in &self.widgets {
            if let Some(id) = child.0.find_id(coord) {
                return Some(id);
            }
        }

        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        for child in &self.widgets {
            child.0.draw(draw_handle, mgr, disabled);
        }
    }
}

impl<W: Widget> event::SendEvent for Grid<W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if !self.is_disabled() {
            for (i, child) in self.widgets.iter_mut().enumerate() {
                if id <= child.0.id() {
                    let r = child.0.send(mgr, id, event);
                    return match Response::try_from(r) {
                        Ok(r) => r,
                        Err(msg) => Response::Msg((i, msg)),
                    };
                }
            }
        }

        Response::Unhandled
    }
}

impl<W: Widget> Grid<W> {
    /// Construct a new instance
    pub fn new(widgets: Vec<(W, GridChildInfo)>) -> Self {
        Grid {
            first_id: Default::default(),
            core: Default::default(),
            widgets,
            data: Default::default(),
        }
    }

    /// Calculate the numbers of columns and rows
    pub fn dimensions(&self) -> (usize, usize) {
        let (mut cols, mut rows) = (0, 0);
        for child in &self.widgets {
            cols = cols.max(child.1.col_end);
            rows = rows.max(child.1.row_end);
        }
        (cols.cast(), rows.cast())
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

    /// Add a widget to a particular cell
    ///
    /// This is just a convenient way to construct a [`GridChildInfo`].
    pub fn add_cell(&mut self, widget: W, col: u32, row: u32) -> TkAction {
        let info = GridChildInfo {
            col,
            col_end: col + 1,
            row,
            row_end: row + 1,
        };
        self.push((widget, info))
    }

    /// Append a child widget
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn push(&mut self, widget: (W, GridChildInfo)) -> TkAction {
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
    pub fn pop(&mut self) -> (Option<(W, GridChildInfo)>, TkAction) {
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
    pub fn insert(&mut self, index: usize, widget: (W, GridChildInfo)) -> TkAction {
        self.widgets.insert(index, widget);
        TkAction::RECONFIGURE
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn remove(&mut self, index: usize) -> ((W, GridChildInfo), TkAction) {
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
    pub fn replace(
        &mut self,
        index: usize,
        mut widget: (W, GridChildInfo),
    ) -> ((W, GridChildInfo), TkAction) {
        std::mem::swap(&mut widget, &mut self.widgets[index]);
        (widget, TkAction::RECONFIGURE)
    }

    /// Append child widgets from an iterator
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widgets
    /// are added.
    pub fn extend<T: IntoIterator<Item = (W, GridChildInfo)>>(&mut self, iter: T) -> TkAction {
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
    pub fn resize_with<F: Fn(usize) -> (W, GridChildInfo)>(
        &mut self,
        len: usize,
        f: F,
    ) -> TkAction {
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
    pub fn retain<F: FnMut(&(W, GridChildInfo)) -> bool>(&mut self, f: F) -> TkAction {
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

    /// Get the index of the child which is an ancestor of `id`, if any
    pub fn find_child_index(&self, id: WidgetId) -> Option<usize> {
        if id >= self.first_id {
            for (i, child) in self.widgets.iter().enumerate() {
                if id <= child.0.id() {
                    return Some(i);
                }
            }
        }
        None
    }
}

impl<W: Widget> Index<usize> for Grid<W> {
    type Output = (W, GridChildInfo);

    fn index(&self, index: usize) -> &Self::Output {
        &self.widgets[index]
    }
}

impl<W: Widget> IndexMut<usize> for Grid<W> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.widgets[index]
    }
}

struct ListIter<'a, W: Widget> {
    list: &'a [(W, GridChildInfo)],
}
impl<'a, W: Widget> Iterator for ListIter<'a, W> {
    type Item = &'a W;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.list.is_empty() {
            let item = &self.list[0].0;
            self.list = &self.list[1..];
            Some(item)
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
