// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with run-time adjustable contents

use std::ops::{Index, IndexMut};

use kas::draw::{DrawHandle, SizeHandle};
use kas::event::{Event, Manager, Response};
use kas::layout::{AxisInfo, RulesSetter, RulesSolver, SizeRules};
use kas::prelude::*;

/// A generic row widget
///
/// See documentation of [`List`] type.
pub type Row<W> = List<kas::Right, W>;

/// A generic column widget
///
/// See documentation of [`List`] type.
pub type Column<W> = List<kas::Down, W>;

/// A row of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type BoxRow<M> = BoxList<kas::Right, M>;

/// A column of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type BoxColumn<M> = BoxList<kas::Down, M>;

/// A row/column of boxed widgets
///
/// This is parameterised over directionality and handler message type.
///
/// See documentation of [`List`] type.
pub type BoxList<D, M> = List<D, Box<dyn Widget<Msg = M>>>;

/// A row of widget references
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type RefRow<'a, M> = RefList<'a, kas::Right, M>;

/// A column of widget references
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type RefColumn<'a, M> = RefList<'a, kas::Down, M>;

/// A row/column of widget references
///
/// This is parameterised over directionality and handler message type.
///
/// See documentation of [`List`] type.
pub type RefList<'a, D, M> = List<D, &'a mut dyn Widget<Msg = M>>;

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
#[handler(send=noauto, msg=<W as event::Handler>::Msg)]
#[widget(children=noauto)]
#[derive(Clone, Default, Debug, Widget)]
pub struct List<D: Directional, W: Widget> {
    #[widget_core]
    core: CoreData,
    widgets: Vec<W>,
    data: layout::DynRowStorage,
    direction: D,
}

impl<D: Directional, W: Widget> WidgetChildren for List<D, W> {
    #[inline]
    fn len(&self) -> usize {
        self.widgets.len()
    }
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn WidgetConfig> {
        self.widgets.get(index).map(|w| w.as_widget())
    }
    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
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

    fn set_rect(&mut self, rect: Rect, _: AlignHints) {
        self.core.rect = rect;
        let dim = (self.direction, self.widgets.len());
        let mut setter = layout::RowSetter::<D, Vec<u32>, _>::new(rect, dim, &mut self.data);

        for (n, child) in self.widgets.iter_mut().enumerate() {
            let align = AlignHints::default();
            child.set_rect(setter.child_rect(&mut self.data, n), align);
        }
    }

    fn spatial_range(&self) -> (usize, usize) {
        let last = WidgetChildren::len(self).wrapping_sub(1);
        match self.direction.is_reversed() {
            false => (0, last),
            true => (last, 0),
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
        solver.for_children(&self.widgets, draw_handle.target_rect(), |w| {
            w.draw(draw_handle, mgr, disabled)
        });
    }
}

impl<D: Directional, W: Widget> event::SendEvent for List<D, W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        for child in &mut self.widgets {
            if id <= child.id() {
                return child.send(mgr, id, event);
            }
        }

        debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
        Manager::handle_generic(self, mgr, event)
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
            core: Default::default(),
            widgets,
            data: Default::default(),
            direction: Default::default(),
        }
    }
}

impl<D: Directional, W: Widget> List<D, W> {
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, widgets: Vec<W>) -> Self {
        List {
            core: Default::default(),
            widgets,
            data: Default::default(),
            direction,
        }
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
            true => TkAction::None,
            false => TkAction::Reconfigure,
        };
        self.widgets.clear();
        action
    }

    /// Append a child widget
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn push(&mut self, widget: W) -> TkAction {
        self.widgets.push(widget);
        TkAction::Reconfigure
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
            true => TkAction::None,
            false => TkAction::Reconfigure,
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
        TkAction::Reconfigure
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn remove(&mut self, index: usize) -> (W, TkAction) {
        let r = self.widgets.remove(index);
        (r, TkAction::Reconfigure)
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
        (widget, TkAction::Reconfigure)
    }

    /// Append child widgets from an iterator
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widgets
    /// are added.
    pub fn extend<T: IntoIterator<Item = W>>(&mut self, iter: T) -> TkAction {
        let len = self.widgets.len();
        self.widgets.extend(iter);
        match len == self.widgets.len() {
            true => TkAction::None,
            false => TkAction::Reconfigure,
        }
    }

    /// Resize, using the given closure to construct new widgets
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn resize_with<F: Fn(usize) -> W>(&mut self, len: usize, f: F) -> TkAction {
        let l0 = self.widgets.len();
        if l0 == len {
            return TkAction::None;
        } else if l0 > len {
            self.widgets.truncate(len);
        } else {
            self.widgets.reserve(len);
            for i in l0..len {
                self.widgets.push(f(i));
            }
        }
        TkAction::Reconfigure
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
            true => TkAction::None,
            false => TkAction::Reconfigure,
        }
    }

    /// Iterate over childern
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a W> {
        ListIter {
            list: self,
            index: 0,
        }
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

struct ListIter<'a, D: Directional, W: Widget> {
    list: &'a List<D, W>,
    index: usize,
}
impl<'a, D: Directional, W: Widget> Iterator for ListIter<'a, D, W> {
    type Item = &'a W;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        if index < self.list.widgets.len() {
            self.index = index + 1;
            Some(&self.list.widgets[index])
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a, D: Directional, W: Widget> ExactSizeIterator for ListIter<'a, D, W> {
    fn len(&self) -> usize {
        self.list.widgets.len() - self.index
    }
}
