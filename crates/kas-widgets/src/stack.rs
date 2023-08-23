// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A stack

use kas::prelude::*;
use std::collections::hash_map::{Entry, HashMap};
use std::fmt::Debug;
use std::ops::{Index, IndexMut, Range};

/// A stack of boxed widgets
///
/// This is a parametrisation of [`Stack`].
pub type BoxStack<Data> = Stack<Box<dyn Widget<Data = Data>>>;

impl_scope! {
    /// A stack of widgets
    ///
    /// A stack consists a set of child widgets, "pages", all of equal size.
    /// Only a single page is visible at a time. The page is "turned" by calling
    /// [`Self::set_active`].
    ///
    /// This may only be parametrised with a single widget type, thus usually
    /// it will be necessary to box children (this is what [`BoxStack`] is).
    ///
    /// Configuring is `O(n)` in the number of pages `n`. Resizing may be `O(n)`
    /// or may be limited: see [`Self::set_size_limit`]. Drawing is `O(1)`, and
    /// so is event handling in the expected case.
    #[autoimpl(Default)]
    #[derive(Clone, Debug)]
    #[widget]
    pub struct Stack<W: Widget> {
        core: widget_core!(),
        widgets: Vec<W>,
        sized_range: Range<usize>, // range of pages for which size rules are solved
        active: usize,
        size_limit: usize,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of WidgetId to index
    }

    impl Widget for Self {
        type Data = W::Data;

        fn for_child_node(
            &mut self,
            data: &W::Data,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get_mut(index) {
                closure(w.as_node(data));
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Layout> {
            self.widgets.get(index).map(|w| w.as_layout())
        }

        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            id.next_key_after(self.id_ref())
                .and_then(|k| self.id_map.get(&k).cloned())
        }

        fn make_child_id(&mut self, index: usize) -> WidgetId {
            if let Some(child) = self.widgets.get(index) {
                // Use the widget's existing identifier, if any
                if child.id_ref().is_valid() {
                    if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                        self.id_map.insert(key, index);
                        return child.id();
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

        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = SizeRules::EMPTY;
            let end = self
                .active
                .saturating_add(self.size_limit)
                .min(self.widgets.len());
            let start = end.saturating_sub(self.size_limit);
            self.sized_range = start..end;
            debug_assert!(self.sized_range.contains(&self.active));
            for index in start..end {
                rules = rules.max(self.widgets[index].size_rules(sizer.re(), axis));
            }
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            if let Some(child) = self.widgets.get_mut(self.active) {
                child.set_rect(cx, rect);
            }
        }

        fn nav_next(&self, _: bool, from: Option<usize>) -> Option<usize> {
            match from {
                None => Some(self.active),
                Some(active) if active != self.active => Some(self.active),
                _ => None,
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            // Latter condition is implied, but compiler doesn't know this:
            if self.sized_range.contains(&self.active) && self.active < self.widgets.len() {
                return self.widgets[self.active].find_id(coord);
            }
            None
        }

        fn draw(&mut self, mut draw: DrawCx) {
            if self.sized_range.contains(&self.active) && self.active < self.widgets.len() {
                draw.recurse(&mut self.widgets[self.active]);
            }
        }
    }

    impl Events for Self {
        fn recurse_range(&self) -> std::ops::Range<usize> {
            self.active..(self.active + 1)
        }

        fn pre_configure(&mut self, _: &mut ConfigCx, id: WidgetId) {
            self.core.id = id;
            self.id_map.clear();
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

impl<W: Widget> Stack<W> {
    /// Construct a new instance
    ///
    /// Initially, the first page (if any) will be shown. Use
    /// [`Self::with_active`] to change this.
    pub fn new(widgets: impl Into<Vec<W>>) -> Self {
        Stack {
            core: Default::default(),
            widgets: widgets.into(),
            sized_range: 0..0,
            active: 0,
            size_limit: usize::MAX,
            next: 0,
            id_map: Default::default(),
        }
    }

    /// Edit the list of children directly
    ///
    /// This may be used to edit pages before window construction. It may
    /// also be used from a running UI, but in this case a full reconfigure
    /// of the window's widgets is required (triggered by the the return
    /// value, [`Action::RECONFIGURE`]).
    #[inline]
    pub fn edit<F: FnOnce(&mut Vec<W>)>(&mut self, f: F) -> Action {
        f(&mut self.widgets);
        Action::RECONFIGURE
    }

    /// Limit the number of pages considered by [`Layout::size_rules`]
    ///
    /// By default, this is `usize::MAX`: all pages affect the result. If
    /// this is set to 1 then only the active page will affect the result. If
    /// this is `n > 1`, then `min(n, num_pages)` pages (including active)
    /// will be used. (If this is set to 0 it is silently replaced with 1.)
    ///
    /// Using a limit lower than the number of pages has two effects:
    /// (1) resizing is faster and (2) calling [`Self::set_active`] may cause a
    /// full-window resize.
    pub fn set_size_limit(&mut self, limit: usize) {
        self.size_limit = limit.max(1);
    }

    /// Get the index of the active widget
    #[inline]
    pub fn active(&self) -> usize {
        self.active
    }

    /// Set the active widget (inline)
    ///
    /// Unlike [`Self::set_active`], this does not update anything; it is
    /// assumed that sizing happens afterwards.
    #[inline]
    pub fn with_active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }

    /// Set the active page
    ///
    /// Behaviour depends on whether [`SizeRules`] were already solved for
    /// `index` (see [`Self::set_size_limit`] and note that methods like
    /// [`Self::push`] do not solve rules for new pages). Case:
    ///
    /// -   `index >= num_pages`: no page displayed
    /// -   `index == active` and `SizeRules` were solved: nothing happens
    /// -   `SizeRules` were solved: set layout ([`Layout::set_rect`]) and
    ///     update mouse-cursor target ([`Action::REGION_MOVED`])
    /// -   Otherwise: resize the whole window ([`Action::RESIZE`])
    pub fn set_active(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize) {
        let old_index = self.active;
        self.active = index;
        if index >= self.widgets.len() {
            if old_index < self.widgets.len() {
                *cx |= Action::REGION_MOVED;
            }
            return;
        }

        if self.sized_range.contains(&index) {
            if old_index != index {
                self.widgets[index].set_rect(cx, self.core.rect);
                *cx |= Action::REGION_MOVED;
            }
        } else {
            *cx |= Action::RESIZE;
        }

        cx.update(self.widgets[index].as_node(data));
    }

    /// Get a direct reference to the active child widget, if any
    pub fn get_active(&self) -> Option<&W> {
        if self.active < self.widgets.len() {
            Some(&self.widgets[self.active])
        } else {
            None
        }
    }

    /// True if there are no pages
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Returns the number of pages
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Remove all pages
    ///
    /// This does not change the activen page index.
    pub fn clear(&mut self) {
        self.widgets.clear();
        self.sized_range = 0..0;
    }

    /// Returns a reference to the page, if any
    pub fn get(&self, index: usize) -> Option<&W> {
        self.widgets.get(index)
    }

    /// Returns a mutable reference to the page, if any
    pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
        self.widgets.get_mut(index)
    }

    /// Append a page
    ///
    /// The new page is configured immediately. If it becomes the active page
    /// and then [`Action::RESIZE`] will be triggered.
    ///
    /// Returns the new page's index.
    pub fn push(&mut self, cx: &mut ConfigCx, data: &W::Data, mut widget: W) -> usize {
        let index = self.widgets.len();
        let id = self.make_child_id(index);
        cx.configure(widget.as_node(data), id);

        self.widgets.push(widget);

        if index == self.active {
            *cx |= Action::RESIZE;
        }

        self.sized_range.end = self.sized_range.end.min(index);
        index
    }

    /// Remove the last child widget (if any) and return
    ///
    /// If this page was active then the previous page becomes active.
    pub fn pop(&mut self, cx: &mut EventState) -> Option<W> {
        let result = self.widgets.pop();
        if let Some(w) = result.as_ref() {
            if self.active > 0 && self.active == self.widgets.len() {
                self.active -= 1;
                if self.sized_range.contains(&self.active) {
                    cx.request_set_rect(self.widgets[self.active].id());
                } else {
                    *cx |= Action::RESIZE;
                }
            }

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
    /// The new child is configured immediately. The active page does not
    /// change.
    pub fn insert(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, mut widget: W) {
        if self.active < index {
            self.sized_range.end = self.sized_range.end.min(index);
        } else {
            self.sized_range.start = (self.sized_range.start + 1).max(index + 1);
            self.sized_range.end += 1;
            self.active = self.active.saturating_add(1);
        }

        let id = self.make_child_id(index);
        cx.configure(widget.as_node(data), id);

        self.widgets.insert(index, widget);

        for v in self.id_map.values_mut() {
            if *v >= index {
                *v += 1;
            }
        }
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// If the active page is removed then the previous page (if any) becomes
    /// active.
    pub fn remove(&mut self, cx: &mut EventState, index: usize) -> W {
        let w = self.widgets.remove(index);
        if w.id_ref().is_valid() {
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        if self.active == index {
            self.active = self.active.saturating_sub(1);
            if self.sized_range.contains(&self.active) {
                cx.request_set_rect(self.widgets[self.active].id());
            } else {
                *cx |= Action::RESIZE;
            }
        }
        if index < self.sized_range.end {
            self.sized_range.end -= 1;
            if index < self.sized_range.start {
                self.sized_range.start -= 1;
            }
        }

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
    /// The new child is configured immediately. If it replaces the active page,
    /// then [`Action::RESIZE`] is triggered.
    pub fn replace(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, mut widget: W) -> W {
        let id = self.make_child_id(index);
        cx.configure(widget.as_node(data), id);
        std::mem::swap(&mut widget, &mut self.widgets[index]);

        if widget.id_ref().is_valid() {
            if let Some(key) = widget.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        if self.active < index {
            self.sized_range.end = self.sized_range.end.min(index);
        } else {
            self.sized_range.start = (self.sized_range.start + 1).max(index + 1);
            self.sized_range.end += 1;
            if index == self.active {
                *cx |= Action::RESIZE;
            }
        }

        widget
    }

    /// Append child widgets from an iterator
    ///
    /// New children are configured immediately. If a new page becomes active,
    /// then [`Action::RESIZE`] is triggered.
    pub fn extend<T: IntoIterator<Item = W>>(
        &mut self,
        cx: &mut ConfigCx,
        data: &W::Data,
        iter: T,
    ) {
        let old_len = self.widgets.len();
        let iter = iter.into_iter();
        if let Some(ub) = iter.size_hint().1 {
            self.widgets.reserve(ub);
        }
        for mut w in iter {
            let id = self.make_child_id(self.widgets.len());
            cx.configure(w.as_node(data), id);
            self.widgets.push(w);
        }

        if (old_len..self.widgets.len()).contains(&self.active) {
            *cx |= Action::RESIZE;
        }
    }

    /// Resize, using the given closure to construct new widgets
    ///
    /// New children are configured immediately. If a new page becomes active,
    /// then [`Action::RESIZE`] is triggered.
    pub fn resize_with<F: Fn(usize) -> W>(
        &mut self,
        cx: &mut ConfigCx,
        data: &W::Data,
        len: usize,
        f: F,
    ) {
        let old_len = self.widgets.len();

        if len < old_len {
            self.sized_range.end = self.sized_range.end.min(len);
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

            if (old_len..len).contains(&self.active) {
                *cx |= Action::RESIZE;
            }
        }
    }
}

impl<W: Widget> FromIterator<W> for Stack<W> {
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = W>,
    {
        Self::new(iter.into_iter().collect::<Vec<W>>())
    }
}
