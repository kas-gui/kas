// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with sizes adjustable via dividing handles

use log::warn;
use std::ops::{Index, IndexMut};

use super::DragHandle;
use kas::dir::{Down, Right};
use kas::event;
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::prelude::*;

/// A generic row widget
///
/// See documentation of [`Splitter`] type.
pub type RowSplitter<W> = Splitter<Right, W>;

/// A generic column widget
///
/// See documentation of [`Splitter`] type.
pub type ColumnSplitter<W> = Splitter<Down, W>;

/// A row of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`Splitter`] type.
pub type BoxRowSplitter<M> = BoxSplitter<Right, M>;

/// A column of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`Splitter`] type.
pub type BoxColumnSplitter<M> = BoxSplitter<Down, M>;

/// A row/column of boxed widgets
///
/// This is parameterised over directionality and handler message type.
///
/// See documentation of [`Splitter`] type.
pub type BoxSplitter<D, M> = Splitter<D, Box<dyn Widget<Msg = M>>>;

/// A row of widget references
///
/// This is parameterised over handler message type.
///
/// See documentation of [`Splitter`] type.
pub type RefRowSplitter<'a, M> = RefSplitter<'a, Right, M>;

/// A column of widget references
///
/// This is parameterised over handler message type.
///
/// See documentation of [`Splitter`] type.
pub type RefColumnSplitter<'a, M> = RefSplitter<'a, Down, M>;

/// A row/column of widget references
///
/// This is parameterised over directionality and handler message type.
///
/// See documentation of [`Splitter`] type.
pub type RefSplitter<'a, D, M> = Splitter<D, &'a mut dyn Widget<Msg = M>>;

widget! {
    /// A resizable row/column widget
    ///
    /// Similar to [`crate::List`] but with draggable handles between items.
    // TODO: better doc
    #[derive(Clone, Default, Debug)]
    #[handler(msg=<W as event::Handler>::Msg)]
    pub struct Splitter<D: Directional, W: Widget> {
        #[widget_core]
        core: CoreData,
        widgets: Vec<W>,
        handles: Vec<DragHandle>,
        data: layout::DynRowStorage,
        direction: D,
    }

    impl WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len() + self.handles.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn WidgetConfig> {
            if (index & 1) != 0 {
                self.handles.get(index >> 1).map(|w| w.as_widget())
            } else {
                self.widgets.get(index >> 1).map(|w| w.as_widget())
            }
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
            if (index & 1) != 0 {
                self.handles.get_mut(index >> 1).map(|w| w.as_widget_mut())
            } else {
                self.widgets.get_mut(index >> 1).map(|w| w.as_widget_mut())
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            if self.widgets.is_empty() {
                return SizeRules::EMPTY;
            }
            assert_eq!(self.handles.len() + 1, self.widgets.len());

            let handle_size = size_handle.separator().extract(axis);

            let dim = (self.direction, self.num_children());
            let mut solver = layout::RowSolver::new(axis, dim, &mut self.data);

            let mut n = 0;
            loop {
                assert!(n < self.widgets.len());
                let widgets = &mut self.widgets;
                solver.for_child(&mut self.data, n << 1, |axis| {
                    widgets[n].size_rules(size_handle, axis)
                });

                if n >= self.handles.len() {
                    break;
                }
                solver.for_child(&mut self.data, (n << 1) + 1, |_axis| {
                    SizeRules::fixed(handle_size, (0, 0))
                });
                n += 1;
            }
            solver.finish(&mut self.data)
        }

        fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            if self.widgets.is_empty() {
                return;
            }
            assert!(self.handles.len() + 1 == self.widgets.len());

            let dim = (self.direction, self.num_children());
            let is_horiz = dim.0.is_horizontal();
            let aa = if is_horiz { align.horiz } else { align.vert };
            if aa.unwrap_or(Align::Stretch) != Align::Stretch {
                warn!("Splitter: found alignment != Stretch");
            }
            let mut setter = layout::RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, &mut self.data);

            let mut n = 0;
            loop {
                assert!(n < self.widgets.len());
                let align = AlignHints::default();
                self.widgets[n].set_rect(mgr, setter.child_rect(&mut self.data, n << 1), align);

                if n >= self.handles.len() {
                    break;
                }

                // TODO(opt): calculate all maximal sizes simultaneously
                let index = (n << 1) + 1;
                let track = setter.maximal_rect_of(&mut self.data, index);
                self.handles[n].set_rect(mgr, track, AlignHints::default());
                let handle = setter.child_rect(&mut self.data, index);
                let _ = self.handles[n].set_size_and_offset(handle.size, handle.pos - track.pos);

                n += 1;
            }
        }

        fn spatial_nav(&mut self, _: &mut Manager, _: bool, _: Option<usize>) -> Option<usize> {
            None // handles are not navigable
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }

            // find_child should gracefully handle the case that a coord is between
            // widgets, so there's no harm (and only a small performance loss) in
            // calling it twice.

            let solver = layout::RowPositionSolver::new(self.direction);
            if let Some(child) = solver.find_child_mut(&mut self.widgets, coord) {
                return child.find_id(coord).or(Some(self.id()));
            }

            let solver = layout::RowPositionSolver::new(self.direction);
            if let Some(child) = solver.find_child_mut(&mut self.handles, coord) {
                return child.find_id(coord).or(Some(self.id()));
            }

            Some(self.id())
        }

        fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            // as with find_id, there's not much harm in invoking the solver twice

            let solver = layout::RowPositionSolver::new(self.direction);
            let disabled = disabled || self.is_disabled();
            solver.for_children(&mut self.widgets, draw.get_clip_rect(), |w| {
                w.draw(draw, mgr, disabled)
            });

            let solver = layout::RowPositionSolver::new(self.direction);
            solver.for_children(&mut self.handles, draw.get_clip_rect(), |w| {
                draw.separator(w.rect())
            });
        }
    }

    impl event::SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if !self.is_disabled() && !self.widgets.is_empty() {
                assert!(self.handles.len() + 1 == self.widgets.len());
                let mut n = 0;
                loop {
                    assert!(n < self.widgets.len());
                    if id <= self.widgets[n].id() {
                        return self.widgets[n].send(mgr, id, event);
                    }

                    if n >= self.handles.len() {
                        break;
                    }
                    if id <= self.handles[n].id() {
                        return self.handles[n]
                            .send(mgr, id, event)
                            .try_into()
                            .unwrap_or_else(|_| {
                                // Message is the new offset relative to the track;
                                // the handle has already adjusted its position
                                self.adjust_size(mgr, n);
                                Response::None
                            });
                    }
                    n += 1;
                }
            }

            Response::Unhandled
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

impl<D: Directional + Default, W: Widget> Splitter<D, W> {
    /// Construct a new instance
    ///
    /// This constructor is available where the direction is determined by the
    /// type: for `D: Directional + Default`. In other cases, use
    /// [`Splitter::new_with_direction`].
    pub fn new(widgets: Vec<W>) -> Self {
        let direction = D::default();
        Self::new_with_direction(direction, widgets)
    }
}

impl<D: Directional, W: Widget> Splitter<D, W> {
    /// Construct a new instance with explicit direction
    pub fn new_with_direction(direction: D, widgets: Vec<W>) -> Self {
        let mut handles = Vec::new();
        handles.resize_with(widgets.len().saturating_sub(1), DragHandle::new);
        Splitter {
            core: Default::default(),
            widgets,
            handles,
            data: Default::default(),
            direction,
        }
    }

    fn adjust_size(&mut self, mgr: &mut Manager, n: usize) {
        assert!(n < self.handles.len());
        assert_eq!(self.widgets.len(), self.handles.len() + 1);
        let index = 2 * n + 1;

        let hrect = self.handles[n].rect();
        let width1 = (hrect.pos - self.core.rect.pos).extract(self.direction);
        let width2 = (self.core.rect.size - hrect.size).extract(self.direction) - width1;

        let dim = (self.direction, self.num_children());
        let mut setter =
            layout::RowSetter::<D, Vec<i32>, _>::new_unsolved(self.core.rect, dim, &mut self.data);
        setter.solve_range(&mut self.data, 0..index, width1);
        setter.solve_range(&mut self.data, (index + 1)..dim.1, width2);
        setter.update_offsets(&mut self.data);

        let mut n = 0;
        loop {
            assert!(n < self.widgets.len());
            let align = AlignHints::default();
            self.widgets[n].set_rect(mgr, setter.child_rect(&mut self.data, n << 1), align);

            if n >= self.handles.len() {
                break;
            }

            let index = (n << 1) + 1;
            let track = self.handles[n].track();
            self.handles[n].set_rect(mgr, track, AlignHints::default());
            let handle = setter.child_rect(&mut self.data, index);
            let _ = self.handles[n].set_size_and_offset(handle.size, handle.pos - track.pos);

            n += 1;
        }
    }

    /// True if there are no child widgets
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Returns the number of child widgets (excluding handles)
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
        self.handles.reserve(additional);
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
        self.handles.clear();
        action
    }

    /// Append a child widget
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn push(&mut self, widget: W) -> TkAction {
        if !self.widgets.is_empty() {
            self.handles.push(DragHandle::new());
        }
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
        let _ = self.handles.pop();
        (self.widgets.pop(), action)
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn insert(&mut self, index: usize, widget: W) -> TkAction {
        if !self.widgets.is_empty() {
            self.handles.push(DragHandle::new());
        }
        self.widgets.insert(index, widget);
        TkAction::RECONFIGURE
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn remove(&mut self, index: usize) -> (W, TkAction) {
        let _ = self.handles.pop();
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
        self.handles
            .resize_with(self.widgets.len().saturating_sub(1), DragHandle::new);
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
        self.handles
            .resize_with(self.widgets.len().saturating_sub(1), DragHandle::new);
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
        self.handles
            .resize_with(self.widgets.len().saturating_sub(1), DragHandle::new);
        match len == self.widgets.len() {
            true => TkAction::empty(),
            false => TkAction::RECONFIGURE,
        }
    }
}
