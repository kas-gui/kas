// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with sizes adjustable via dividing handles

use std::collections::hash_map::{Entry, HashMap};
use std::ops::{Index, IndexMut};

use super::{GripMsg, GripPart};
use kas::dir::{Down, Right};
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::Feature;

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
pub type BoxRowSplitter = BoxSplitter<Right>;

/// A column of boxed widgets
///
/// This is parameterised over handler message type.
///
/// See documentation of [`Splitter`] type.
pub type BoxColumnSplitter = BoxSplitter<Down>;

/// A row/column of boxed widgets
///
/// This is parameterised over directionality.
///
/// See documentation of [`Splitter`] type.
pub type BoxSplitter<D> = Splitter<D, Box<dyn Widget>>;

/// A row of widget references
///
/// See documentation of [`Splitter`] type.
pub type RefRowSplitter<'a> = RefSplitter<'a, Right>;

/// A column of widget references
///
/// See documentation of [`Splitter`] type.
pub type RefColumnSplitter<'a> = RefSplitter<'a, Down>;

/// A row/column of widget references
///
/// This is parameterised over directionality.
///
/// See documentation of [`Splitter`] type.
pub type RefSplitter<'a, D> = Splitter<D, &'a mut dyn Widget>;

impl_scope! {
    /// A resizable row/column widget
    ///
    /// Similar to [`crate::List`] but with draggable handles between items.
    // TODO: better doc
    #[derive(Clone, Default, Debug)]
    #[widget]
    pub struct Splitter<D: Directional, W: Widget> {
        core: widget_core!(),
        widgets: Vec<W>,
        handles: Vec<GripPart>,
        data: layout::DynRowStorage,
        direction: D,
        size_solved: bool,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of WidgetId to index
    }

    impl Self {
        // Assumption: index is a valid entry of self.widgets
        fn make_next_id(&mut self, is_handle: bool, index: usize) -> WidgetId {
            let child_index = (2 * index) + (is_handle as usize);
            if !is_handle {
                if let Some(child) = self.widgets.get(index) {
                    // Use the widget's existing identifier, if any
                    if child.id_ref().is_valid() {
                        if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                            self.id_map.insert(key, child_index);
                            return child.id();
                        }
                    }
                }
            }

            loop {
                let key = self.next;
                self.next += 1;
                if let Entry::Vacant(entry) = self.id_map.entry(key) {
                    entry.insert(child_index);
                    return self.id_ref().make_child(key);
                }
            }
        }
    }

    impl WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len() + self.handles.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn Widget> {
            if (index & 1) != 0 {
                self.handles.get(index >> 1).map(|w| w.as_widget())
            } else {
                self.widgets.get(index >> 1).map(|w| w.as_widget())
            }
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
            if (index & 1) != 0 {
                self.handles.get_mut(index >> 1).map(|w| w.as_widget_mut())
            } else {
                self.widgets.get_mut(index >> 1).map(|w| w.as_widget_mut())
            }
        }

        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            id.next_key_after(self.id_ref()).and_then(|k| self.id_map.get(&k).cloned())
        }

        fn make_child_id(&mut self, child_index: usize) -> WidgetId {
            let is_handle = (child_index & 1) != 0;
            self.make_next_id(is_handle, child_index / 2)
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            if self.widgets.is_empty() {
                return SizeRules::EMPTY;
            }
            assert_eq!(self.handles.len() + 1, self.widgets.len());

            let handle_rules = size_mgr.feature(Feature::Separator, axis);

            let dim = (self.direction, self.num_children());
            let mut solver = layout::RowSolver::new(axis, dim, &mut self.data);

            let mut n = 0;
            loop {
                assert!(n < self.widgets.len());
                let widgets = &mut self.widgets;
                solver.for_child(&mut self.data, n << 1, |axis| {
                    widgets[n].size_rules(size_mgr.re(), axis)
                });

                if n >= self.handles.len() {
                    break;
                }
                solver.for_child(&mut self.data, (n << 1) + 1, |_axis| handle_rules);
                n += 1;
            }
            solver.finish(&mut self.data)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            self.size_solved = true;
            if self.widgets.is_empty() {
                return;
            }
            assert!(self.handles.len() + 1 == self.widgets.len());

            let dim = (self.direction, self.num_children());
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

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) || !self.size_solved {
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

        fn draw(&mut self, mut draw: DrawMgr) {
            if !self.size_solved {
                return;
            }
            // as with find_id, there's not much harm in invoking the solver twice

            let solver = layout::RowPositionSolver::new(self.direction);
            solver.for_children(&mut self.widgets, draw.get_clip_rect(), |w| {
                draw.recurse(w);
            });

            let solver = layout::RowPositionSolver::new(self.direction);
            solver.for_children(&mut self.handles, draw.get_clip_rect(), |w| {
                draw.separator(w.rect())
            });
        }
    }

    impl Widget for Self {
        fn pre_configure(&mut self, _: &mut ConfigMgr, id: WidgetId) {
            self.core.id = id;
            self.id_map.clear();
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
            if (index & 1) == 1 {
                if let Some(GripMsg::PressMove(offset)) = mgr.try_pop_msg() {
                    let n = index >> 1;
                    assert!(n < self.handles.len());
                    *mgr |= self.handles[n].set_offset(offset).1;
                    mgr.config_mgr(|mgr| self.adjust_size(mgr, n));
                }
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
        handles.resize_with(widgets.len().saturating_sub(1), GripPart::new);
        Splitter {
            core: Default::default(),
            widgets,
            handles,
            data: Default::default(),
            direction,
            size_solved: false,
            next: 0,
            id_map: Default::default(),
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
        let len = self.widgets.len().saturating_sub(1);
        self.handles.resize_with(len, GripPart::new);
        TkAction::RECONFIGURE
    }

    fn adjust_size(&mut self, mgr: &mut ConfigMgr, n: usize) {
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

    /// Remove all child widgets
    pub fn clear(&mut self) {
        self.widgets.clear();
        self.handles.clear();
        self.size_solved = false;
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
    /// The new child is configured immediately. [`TkAction::RESIZE`] is
    /// triggered.
    ///
    /// Returns the new element's index.
    pub fn push(&mut self, mgr: &mut ConfigMgr, widget: W) -> usize {
        let index = self.widgets.len();
        if index > 0 {
            let len = self.handles.len();
            self.handles.push(GripPart::new());
            let id = self.make_next_id(true, len);
            mgr.configure(id, &mut self.handles[len]);
        }
        self.widgets.push(widget);
        let id = self.make_next_id(false, index);
        mgr.configure(id, &mut self.widgets[index]);
        self.size_solved = false;
        *mgr |= TkAction::RESIZE;
        index
    }

    /// Remove the last child widget (if any) and return
    ///
    /// Triggers [`TkAction::RESIZE`].
    pub fn pop(&mut self, mgr: &mut ConfigMgr) -> Option<W> {
        let result = self.widgets.pop();
        if let Some(w) = result.as_ref() {
            *mgr |= TkAction::RESIZE;

            if w.id_ref().is_valid() {
                if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                    self.id_map.remove(&key);
                }
            }

            if let Some(w) = self.handles.pop() {
                if w.id_ref().is_valid() {
                    if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                        self.id_map.remove(&key);
                    }
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
    pub fn insert(&mut self, mgr: &mut ConfigMgr, index: usize, widget: W) {
        for v in self.id_map.values_mut() {
            if *v >= index {
                *v += 2;
            }
        }

        if !self.widgets.is_empty() {
            let index = index.min(self.handles.len());
            self.handles.insert(index, GripPart::new());
            let id = self.make_next_id(true, index);
            mgr.configure(id, &mut self.handles[index]);
        }

        self.widgets.insert(index, widget);
        let id = self.make_next_id(false, index);
        mgr.configure(id, &mut self.widgets[index]);

        self.size_solved = false;
        *mgr |= TkAction::RESIZE;
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers [`TkAction::RESIZE`].
    pub fn remove(&mut self, mgr: &mut ConfigMgr, index: usize) -> W {
        if !self.handles.is_empty() {
            let index = index.min(self.handles.len());
            let w = self.handles.remove(index);
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        let w = self.widgets.remove(index);
        if w.id_ref().is_valid() {
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        *mgr |= TkAction::RESIZE;

        for v in self.id_map.values_mut() {
            if *v > index {
                *v -= 2;
            }
        }
        w
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// The new child is configured immediately. Triggers [`TkAction::RESIZE`].
    pub fn replace(&mut self, mgr: &mut ConfigMgr, index: usize, mut w: W) -> W {
        std::mem::swap(&mut w, &mut self.widgets[index]);

        if w.id_ref().is_valid() {
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        let id = self.make_next_id(false, index);
        mgr.configure(id, &mut self.widgets[index]);

        self.size_solved = false;
        *mgr |= TkAction::RESIZE;

        w
    }
}
