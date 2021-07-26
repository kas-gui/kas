// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with run-time adjustable contents

use std::ops::{Index, IndexMut};

use kas::dir::{Down, Right};
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::{event, prelude::*};

/// A generic row widget
///
/// See documentation of [`List`] type.
pub type Row<W> = List<Right, W>;

/// A generic column widget
///
/// See documentation of [`List`] type.
pub type Column<W> = List<Down, W>;

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
/// This type is generic over both directionality and the type of child widgets.
/// Essentially, it is a [`Vec`] which also implements the [`Widget`] trait.
///
/// [`Row`] and [`Column`] are parameterisations with set directionality.
///
/// [`BoxList`] (and its derivatives [`BoxRow`], [`BoxColumn`]) parameterise
/// `W = Box<dyn Widget>`, thus supporting individually boxed child widgets.
/// This allows use of multiple types of child widget at the cost of extra
/// allocation, and requires dynamic dispatch of methods.
///
/// Configuring and resizing elements is O(n) in the number of children.
/// Drawing and event handling is O(log n) in the number of children (assuming
/// only a small number are visible at any one time).
///
/// For fixed configurations of child widgets, [`make_widget`] can be used
/// instead. [`make_widget`] has the advantage that it can support child widgets
/// of multiple types without allocation and via static dispatch, but the
/// disadvantage that drawing and event handling are O(n) in the number of
/// children.
///
/// [`make_widget`]: ../macros/index.html#the-make_widget-macro
#[derive(Clone, Default, Debug, Widget)]
#[handler(send=noauto, msg=(usize, <W as event::Handler>::Msg))]
#[widget(children=noauto)]
pub struct List<D: Directional, W: Widget> {
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    widgets: Vec<W>,
    data: layout::DynRowStorage,
    direction: D,
}

impl<D: Directional, W: Widget> WidgetChildren for List<D, W> {
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

impl<D: Directional, W: Widget> Layout for List<D, W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.widgets.len());
        let mut solver = layout::RowSolver::new(axis, dim, &mut self.data);
        for (n, child) in self.widgets.iter_mut().enumerate() {
            solver.for_child(&mut self.data, n, |axis| {
                child.size_rules(size_handle, axis)
            });
        }
        solver.finish(&mut self.data)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let dim = (self.direction, self.widgets.len());
        let mut setter = layout::RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, &mut self.data);

        for (n, child) in self.widgets.iter_mut().enumerate() {
            child.set_rect(mgr, setter.child_rect(&mut self.data, n), align);
        }
    }

    fn spatial_nav(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
        if self.num_children() == 0 {
            return None;
        }

        let last = self.num_children() - 1;
        let reverse = reverse ^ self.direction.is_reversed();

        if let Some(index) = from {
            match reverse {
                false if index < last => Some(index + 1),
                true if 0 < index => Some(index - 1),
                _ => None,
            }
        } else {
            match reverse {
                false => Some(0),
                true => Some(last),
            }
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        let solver = layout::RowPositionSolver::new(self.direction);
        if let Some(child) = solver.find_child(&self.widgets, coord) {
            return child.find_id(coord);
        }

        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        let solver = layout::RowPositionSolver::new(self.direction);
        solver.for_children(&self.widgets, draw_handle.clip_rect(), |w| {
            w.draw(draw_handle, mgr, disabled)
        });
    }
}

impl<D: Directional, W: Widget> event::SendEvent for List<D, W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if !self.is_disabled() {
            for (i, child) in self.widgets.iter_mut().enumerate() {
                if id <= child.id() {
                    let r = child.send(mgr, id, event);
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

impl<D: Directional + Default, W: Widget> List<D, W> {
    /// Construct a new instance
    ///
    /// This constructor is available where the direction is determined by the
    /// type: for `D: Directional + Default`. In other cases, use
    /// [`List::new_with_direction`].
    pub fn new(widgets: Vec<W>) -> Self {
        List {
            first_id: Default::default(),
            core: Default::default(),
            widgets,
            data: Default::default(),
            direction: Default::default(),
        }
    }
}

impl<W: Widget> List<Direction, W> {
    /// Set the direction of contents
    pub fn set_direction(&mut self, direction: Direction) -> TkAction {
        self.direction = direction;
        // Note: most of the time SET_SIZE would be enough, but margins can be different
        TkAction::RESIZE
    }
}

impl<D: Directional, W: Widget> List<D, W> {
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, widgets: Vec<W>) -> Self {
        List {
            first_id: Default::default(),
            core: Default::default(),
            widgets,
            data: Default::default(),
            direction,
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

impl<D: Directional, W: Widget> Index<usize> for List<D, W> {
    type Output = W;

    fn index(&self, index: usize) -> &Self::Output {
        &self.widgets[index]
    }
}

impl<D: Directional, W: Widget> IndexMut<usize> for List<D, W> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.widgets[index]
    }
}

struct ListIter<'a, W: Widget> {
    list: &'a [W],
}
impl<'a, W: Widget> Iterator for ListIter<'a, W> {
    type Item = &'a W;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.list.is_empty() {
            let item = &self.list[0];
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
