// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dynamic widgets

use std::ops::{Index, IndexMut};

use kas::draw::{DrawHandle, SizeHandle};
use kas::event::{Event, Manager, Response};
use kas::layout::{AxisInfo, RulesSetter, RulesSolver, SizeRules};
use kas::prelude::*;

/// A generic row widget
///
/// See documentation of [`List`] type.
pub type Row<W> = List<Horizontal, W>;

/// A generic column widget
///
/// See documentation of [`List`] type.
pub type Column<W> = List<Vertical, W>;

/// A row of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type BoxRow<M> = BoxList<Horizontal, M>;

/// A column of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`List`] type.
pub type BoxColumn<M> = BoxList<Vertical, M>;

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
#[derive(Clone, Default, Debug)]
pub struct List<D: Directional, W: Widget> {
    core: CoreData,
    widgets: Vec<W>,
    data: layout::DynRowStorage,
    direction: D,
}

impl<D: Directional, W: Widget> WidgetConfig for List<D, W> {}

// We implement this manually, because the derive implementation cannot handle
// vectors of child widgets.
impl<D: Directional, W: Widget> WidgetCore for List<D, W> {
    #[inline]
    fn core_data(&self) -> &CoreData {
        &self.core
    }
    #[inline]
    fn core_data_mut(&mut self) -> &mut CoreData {
        &mut self.core
    }

    #[inline]
    fn widget_name(&self) -> &'static str {
        "List"
    }

    #[inline]
    fn as_widget(&self) -> &dyn WidgetConfig {
        self
    }
    #[inline]
    fn as_widget_mut(&mut self) -> &mut dyn WidgetConfig {
        self
    }

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

    fn walk(&self, f: &mut dyn FnMut(&dyn WidgetConfig)) {
        for child in &self.widgets {
            child.walk(f);
        }
        f(self)
    }
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn WidgetConfig)) {
        for child in &mut self.widgets {
            child.walk_mut(f);
        }
        f(self)
    }
}

impl<D: Directional, W: Widget> Layout for List<D, W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut solver = layout::RowSolver::<Vec<u32>, _>::new(
            axis,
            (self.direction, self.widgets.len()),
            &mut self.data,
        );
        for (n, child) in self.widgets.iter_mut().enumerate() {
            solver.for_child(&mut self.data, n, |axis| {
                child.size_rules(size_handle, axis)
            });
        }
        solver.finish(&mut self.data)
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, _: AlignHints) {
        self.core.rect = rect;
        let mut setter = layout::RowSetter::<D, Vec<u32>, _>::new(
            rect,
            (self.direction, self.widgets.len()),
            &mut self.data,
        );

        for (n, child) in self.widgets.iter_mut().enumerate() {
            let align = AlignHints::default();
            child.set_rect(size_handle, setter.child_rect(n), align);
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        let solver = layout::RowPositionSolver::new(self.direction);
        if let Some(child) = solver.find_child(&self.widgets, coord) {
            return child.find_id(coord);
        }

        // We should return Some(self), but hit a borrow check error.
        // This should however be unreachable anyway.
        None
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        let solver = layout::RowPositionSolver::new(self.direction);
        solver.for_children(&self.widgets, draw_handle.target_rect(), |w| {
            w.draw(draw_handle, mgr)
        });
    }
}

impl<D: Directional, W: Widget> event::Handler for List<D, W> {
    type Msg = <W as event::Handler>::Msg;
}

impl<D: Directional, W: Widget> event::EventHandler for List<D, W> {
    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        for child in &mut self.widgets {
            if id <= child.id() {
                return child.event(mgr, id, event);
            }
        }
        debug_assert!(id == self.id(), "Handler::handle: bad WidgetId");
        Response::Unhandled(event)
    }
}

impl<D: Directional, W: Widget> Widget for List<D, W> {}

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
    pub fn clear(&mut self, mgr: &mut Manager) {
        if !self.widgets.is_empty() {
            mgr.send_action(TkAction::Reconfigure);
        }
        self.widgets.clear();
    }

    /// Append a child widget
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn push(&mut self, mgr: &mut Manager, widget: W) {
        self.widgets.push(widget);
        mgr.send_action(TkAction::Reconfigure);
    }

    /// Remove the last child widget
    ///
    /// Returns `None` if there are no children. Otherwise, this
    /// triggers a reconfigure before the next draw operation.
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widget is
    /// removed.
    pub fn pop(&mut self, mgr: &mut Manager) -> Option<W> {
        if !self.widgets.is_empty() {
            mgr.send_action(TkAction::Reconfigure);
        }
        self.widgets.pop()
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn insert(&mut self, mgr: &mut Manager, index: usize, widget: W) {
        self.widgets.insert(index, widget);
        mgr.send_action(TkAction::Reconfigure);
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn remove(&mut self, mgr: &mut Manager, index: usize) -> W {
        let r = self.widgets.remove(index);
        mgr.send_action(TkAction::Reconfigure);
        r
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    // TODO: in theory it is possible to avoid a reconfigure where both widgets
    // have no children and have compatible size. Is this a good idea and can
    // we somehow test "has compatible size"?
    pub fn replace(&mut self, mgr: &mut Manager, index: usize, mut widget: W) -> W {
        std::mem::swap(&mut widget, &mut self.widgets[index]);
        mgr.send_action(TkAction::Reconfigure);
        widget
    }

    /// Append child widgets from an iterator
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widgets
    /// are added.
    pub fn extend<T: IntoIterator<Item = W>>(&mut self, mgr: &mut Manager, iter: T) {
        let len = self.widgets.len();
        self.widgets.extend(iter);
        if len != self.widgets.len() {
            mgr.send_action(TkAction::Reconfigure);
        }
    }

    /// Resize, using the given closure to construct new widgets
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn resize_with<F: Fn(usize) -> W>(&mut self, mgr: &mut Manager, len: usize, f: F) {
        let l0 = self.widgets.len();
        if l0 == len {
            return;
        } else if l0 > len {
            self.widgets.truncate(len);
        } else {
            self.widgets.reserve(len);
            for i in l0..len {
                self.widgets.push(f(i));
            }
        }
        mgr.send_action(TkAction::Reconfigure);
    }

    /// Retain only widgets satisfying predicate `f`
    ///
    /// See documentation of [`Vec::retain`].
    ///
    /// Triggers a [reconfigure action](Manager::send_action) if any widgets
    /// are removed.
    pub fn retain<F: FnMut(&W) -> bool>(&mut self, mgr: &mut Manager, f: F) {
        let len = self.widgets.len();
        self.widgets.retain(f);
        if len != self.widgets.len() {
            mgr.send_action(TkAction::Reconfigure);
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
