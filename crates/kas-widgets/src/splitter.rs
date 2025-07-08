// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A row or column with sizes adjustable via dividing grips

use std::collections::hash_map::{Entry, HashMap};
use std::ops::{Index, IndexMut};

use super::{GripMsg, GripPart};
use kas::Collection;
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::Feature;

#[impl_self]
mod Splitter {
    /// A resizable row/column widget
    ///
    /// Similar to [`crate::List`] but with draggable grips between items.
    // TODO: better doc
    #[derive(Clone, Default, Debug)]
    #[widget]
    pub struct Splitter<C: Collection, D: Directional = Direction> {
        core: widget_core!(),
        align_hints: AlignHints,
        widgets: C,
        grips: Vec<GripPart>,
        data: layout::DynRowStorage,
        direction: D,
        size_solved: bool,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of Id to index
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a new instance with default-constructed direction
        #[inline]
        pub fn new(widgets: C) -> Self {
            Self::new_dir(widgets, Default::default())
        }
    }
    impl<C: Collection> Splitter<C, kas::dir::Left> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn left(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> Splitter<C, kas::dir::Right> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn right(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> Splitter<C, kas::dir::Up> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn up(widgets: C) -> Self {
            Self::new(widgets)
        }
    }
    impl<C: Collection> Splitter<C, kas::dir::Down> {
        /// Construct a new instance with fixed direction
        #[inline]
        pub fn down(widgets: C) -> Self {
            Self::new(widgets)
        }
    }

    impl<C: Collection> Splitter<C, Direction> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, cx: &mut EventState, direction: Direction) {
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
            let mut grips = Vec::new();
            grips.resize_with(widgets.len().saturating_sub(1), GripPart::new);
            Splitter {
                core: Default::default(),
                align_hints: AlignHints::NONE,
                widgets,
                grips,
                data: Default::default(),
                direction,
                size_solved: false,
                next: 0,
                id_map: Default::default(),
            }
        }

        // Assumption: index is a valid entry of self.widgets
        fn make_next_id(&mut self, is_grip: bool, index: usize) -> Id {
            let child_index = (2 * index) + (is_grip as usize);
            if !is_grip {
                if let Some(child) = self.widgets.get_tile(index) {
                    // Use the widget's existing identifier, if valid
                    if child.id_ref().is_valid() && self.id_ref().is_ancestor_of(child.id_ref()) {
                        if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                            if let Entry::Vacant(entry) = self.id_map.entry(key) {
                                entry.insert(child_index);
                                return child.id();
                            }
                        }
                    }
                }
            } else {
                if let Some(child) = self.grips.get_tile(index) {
                    // Use the widget's existing identifier, if valid
                    if child.id_ref().is_valid() && self.id_ref().is_ancestor_of(child.id_ref()) {
                        if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                            if let Entry::Vacant(entry) = self.id_map.entry(key) {
                                entry.insert(child_index);
                                return child.id();
                            }
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

        #[inline]
        fn dim(&self) -> (D, usize) {
            (self.direction, self.widgets.len() + self.grips.len())
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            if self.widgets.is_empty() {
                return SizeRules::EMPTY;
            }
            assert_eq!(self.grips.len() + 1, self.widgets.len());

            let grip_rules = sizer.feature(Feature::Separator, axis);

            let mut solver = layout::RowSolver::new(axis, self.dim(), &mut self.data);

            let mut n = 0;
            loop {
                assert!(n < self.widgets.len());
                let widgets = &mut self.widgets;
                if let Some(w) = widgets.get_mut_tile(n) {
                    solver.for_child(&mut self.data, n << 1, |axis| {
                        w.size_rules(sizer.re(), axis)
                    });
                }

                if n >= self.grips.len() {
                    break;
                }
                let grips = &mut self.grips;
                solver.for_child(&mut self.data, (n << 1) + 1, |axis| {
                    grips[n].size_rules(sizer.re(), axis);
                    grip_rules
                });
                n += 1;
            }
            solver.finish(&mut self.data)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.align_hints = hints;
            self.size_solved = true;
            if self.widgets.is_empty() {
                return;
            }
            assert!(self.grips.len() + 1 == self.widgets.len());

            let mut setter =
                layout::RowSetter::<D, Vec<i32>, _>::new(rect, self.dim(), &mut self.data);

            let mut n = 0;
            loop {
                assert!(n < self.widgets.len());
                if let Some(w) = self.widgets.get_mut_tile(n) {
                    w.set_rect(cx, setter.child_rect(&mut self.data, n << 1), hints);
                }

                if n >= self.grips.len() {
                    break;
                }

                // TODO(opt): calculate all maximal sizes simultaneously
                let index = (n << 1) + 1;
                let track = setter.maximal_rect_of(&mut self.data, index);
                self.grips[n].set_track(track);
                let rect = setter.child_rect(&mut self.data, index);
                self.grips[n].set_rect(cx, rect, AlignHints::NONE);

                n += 1;
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            if !self.size_solved {
                debug_assert!(false);
                return;
            }
            // find_child should gracefully handle the case that a coord is between
            // widgets, so there's no harm (and only a small performance loss) in
            // calling it twice.

            let solver = layout::RowPositionSolver::new(self.direction);
            solver.for_children(&self.widgets, draw.get_clip_rect(), |w| {
                w.draw(draw.re());
            });

            let solver = layout::RowPositionSolver::new(self.direction);
            solver.for_children(&self.grips, draw.get_clip_rect(), |w| {
                draw.separator(w.rect())
            });
        }
    }

    impl Tile for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len() + self.grips.len()
        }
        #[inline]
        fn child_indices(&self) -> ChildIndices {
            (0..self.widgets.len() + self.grips.len()).into()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            if (index & 1) != 0 {
                self.grips.get(index >> 1).map(|w| w.as_tile())
            } else {
                self.widgets.get_tile(index >> 1)
            }
        }

        fn find_child_index(&self, id: &Id) -> Option<usize> {
            id.next_key_after(self.id_ref())
                .and_then(|k| self.id_map.get(&k).cloned())
        }

        fn probe(&self, coord: Coord) -> Id {
            if !self.size_solved {
                debug_assert!(false);
                return self.id();
            }

            // find_child should gracefully handle the case that a coord is between
            // widgets, so there's no harm (and only a small performance loss) in
            // calling it twice.

            let solver = layout::RowPositionSolver::new(self.direction);
            if let Some(child) = solver.find_child(&self.widgets, coord) {
                return child.try_probe(coord).unwrap_or_else(|| self.id());
            }

            let solver = layout::RowPositionSolver::new(self.direction);
            if let Some(child) = solver.find_child(&self.grips, coord) {
                return child.try_probe(coord).unwrap_or_else(|| self.id());
            }

            self.id()
        }
    }

    impl Events for Self {
        fn make_child_id(&mut self, child_index: usize) -> Id {
            let is_grip = (child_index & 1) != 0;
            self.make_next_id(is_grip, child_index / 2)
        }

        fn configure(&mut self, _: &mut ConfigCx) {
            // All children will be re-configured which will rebuild id_map
            self.id_map.clear();
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(index) = cx.last_child() {
                if (index & 1) == 1 {
                    if let Some(GripMsg::PressMove(mut offset)) = cx.try_pop() {
                        let n = index >> 1;
                        assert!(n < self.grips.len());
                        if let Some(grip) = self.grips.get_mut(n) {
                            if self.direction.is_reversed() {
                                offset = Offset::conv(grip.track().size) - offset;
                            }
                            grip.set_offset(cx, offset);
                        }
                        self.adjust_size(&mut cx.config_cx(), n);
                    }
                }
            }
        }
    }

    impl Widget for Self {
        type Data = C::Data;

        fn child_node<'n>(&'n mut self, data: &'n C::Data, index: usize) -> Option<Node<'n>> {
            if (index & 1) != 0 {
                self.grips.get_mut(index >> 1).map(|w| w.as_node(&()))
            } else {
                self.widgets.child_node(data, index >> 1)
            }
        }
    }
}

impl<C: Collection, D: Directional> Splitter<C, D> {
    fn adjust_size(&mut self, cx: &mut ConfigCx, n: usize) {
        assert!(n < self.grips.len());
        assert_eq!(self.widgets.len(), self.grips.len() + 1);
        let index = 2 * n + 1;

        let hrect = self.grips[n].rect();
        let width1 = (hrect.pos - self.rect().pos).extract(self.direction);
        let width2 = (self.rect().size - hrect.size).extract(self.direction) - width1;

        let dim = self.dim();
        let mut setter =
            layout::RowSetter::<D, Vec<i32>, _>::new_unsolved(self.rect(), dim, &mut self.data);
        setter.solve_range(&mut self.data, 0..index, width1);
        setter.solve_range(&mut self.data, (index + 1)..dim.1, width2);
        setter.update_offsets(&mut self.data);

        let mut n = 0;
        loop {
            assert!(n < self.widgets.len());
            if let Some(w) = self.widgets.get_mut_tile(n) {
                let rect = setter.child_rect(&mut self.data, n << 1);
                w.set_rect(cx, rect, self.align_hints);
            }

            if n >= self.grips.len() {
                break;
            }

            let index = (n << 1) + 1;
            let track = self.grips[n].track();
            self.grips[n].set_track(track);
            let rect = setter.child_rect(&mut self.data, index);
            self.grips[n].set_rect(cx, rect, AlignHints::NONE);

            n += 1;
        }
    }

    /// True if there are no child widgets
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Returns the number of child widgets (excluding grips)
    pub fn len(&self) -> usize {
        self.widgets.len()
    }
}

impl<W: Widget, D: Directional> Index<usize> for Splitter<Vec<W>, D> {
    type Output = W;

    fn index(&self, index: usize) -> &Self::Output {
        &self.widgets[index]
    }
}

impl<W: Widget, D: Directional> IndexMut<usize> for Splitter<Vec<W>, D> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.widgets[index]
    }
}

impl<W: Widget, D: Directional> Splitter<Vec<W>, D> {
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
        self.grips.clear();
        self.size_solved = false;
    }

    /// Append a child widget
    ///
    /// The new child is configured immediately. [`Action::RESIZE`] is
    /// triggered.
    ///
    /// Returns the new element's index.
    pub fn push(&mut self, cx: &mut ConfigCx, data: &W::Data, mut widget: W) -> usize {
        let index = self.widgets.len();
        if index > 0 {
            let len = self.grips.len();
            let id = self.make_next_id(true, len);
            let mut w = GripPart::new();
            cx.configure(w.as_node(&()), id);
            self.grips.push(w);
        }

        let id = self.make_next_id(false, index);
        cx.configure(widget.as_node(data), id);
        self.widgets.push(widget);

        self.size_solved = false;
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

            if let Some(w) = self.grips.pop() {
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
    /// The new child is configured immediately. Triggers [`Action::RESIZE`].
    pub fn insert(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, mut widget: W) {
        for v in self.id_map.values_mut() {
            if *v >= index {
                *v += 2;
            }
        }

        if !self.widgets.is_empty() {
            let index = index.min(self.grips.len());
            let id = self.make_next_id(true, index);
            let mut w = GripPart::new();
            cx.configure(w.as_node(&()), id);
            self.grips.insert(index, w);
        }

        let id = self.make_next_id(false, index);
        cx.configure(widget.as_node(data), id);
        self.widgets.insert(index, widget);

        self.size_solved = false;
        cx.resize(self);
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// Triggers [`Action::RESIZE`].
    pub fn remove(&mut self, cx: &mut EventState, index: usize) -> W {
        if !self.grips.is_empty() {
            let index = index.min(self.grips.len());
            let w = self.grips.remove(index);
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

        cx.resize(&self);

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
    /// The new child is configured immediately. Triggers [`Action::RESIZE`].
    pub fn replace(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, mut w: W) -> W {
        let id = self.make_next_id(false, index);
        cx.configure(w.as_node(data), id);
        std::mem::swap(&mut w, &mut self.widgets[index]);

        if w.id_ref().is_valid() {
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        self.size_solved = false;
        cx.resize(self);

        w
    }

    /// Append child widgets from an iterator
    ///
    /// New children are configured immediately. Triggers [`Action::RESIZE`].
    pub fn extend<T: IntoIterator<Item = W>>(
        &mut self,
        data: &W::Data,
        cx: &mut ConfigCx,
        iter: T,
    ) {
        let iter = iter.into_iter();
        if let Some(ub) = iter.size_hint().1 {
            self.grips.reserve(ub);
            self.widgets.reserve(ub);
        }

        for mut widget in iter {
            let index = self.widgets.len();
            if index > 0 {
                let id = self.make_next_id(true, self.grips.len());
                let mut w = GripPart::new();
                cx.configure(w.as_node(&()), id);
                self.grips.push(w);
            }

            let id = self.make_next_id(false, index);
            cx.configure(widget.as_node(data), id);
            self.widgets.push(widget);
        }

        self.size_solved = false;
        cx.resize(self);
    }

    /// Resize, using the given closure to construct new widgets
    ///
    /// New children are configured immediately. Triggers [`Action::RESIZE`].
    pub fn resize_with<F: Fn(usize) -> W>(
        &mut self,
        data: &W::Data,
        cx: &mut ConfigCx,
        len: usize,
        f: F,
    ) {
        let old_len = self.widgets.len();

        if len < old_len {
            cx.resize(&self);
            loop {
                let result = self.widgets.pop();
                if let Some(w) = result.as_ref() {
                    if w.id_ref().is_valid() {
                        if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                            self.id_map.remove(&key);
                        }
                    }

                    if let Some(w) = self.grips.pop() {
                        if w.id_ref().is_valid() {
                            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                                self.id_map.remove(&key);
                            }
                        }
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
                if index > 0 {
                    let id = self.make_next_id(true, self.grips.len());
                    let mut w = GripPart::new();
                    cx.configure(w.as_node(&()), id);
                    self.grips.push(w);
                }

                let id = self.make_next_id(false, index);
                let mut widget = f(index);
                cx.configure(widget.as_node(data), id);
                self.widgets.push(widget);
            }

            self.size_solved = false;
            cx.resize(self);
        }
    }
}
