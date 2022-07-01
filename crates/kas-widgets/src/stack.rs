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
pub type BoxStack = Stack<Box<dyn Widget>>;

/// A stack of widget references
///
/// This is a parametrisation of [`Stack`].
pub type RefStack<'a> = Stack<&'a mut dyn Widget>;

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
        align_hints: AlignHints,
        widgets: Vec<W>,
        sized_range: Range<usize>, // range of pages for which size rules are solved
        active: usize,
        size_limit: usize,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of WidgetId to index
    }

    impl WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn Widget> {
            self.widgets.get(index).map(|w| w.as_widget())
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(index).map(|w| w.as_widget_mut())
        }

        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            id.next_key_after(self.id_ref()).and_then(|k| self.id_map.get(&k).cloned())
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
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let mut rules = SizeRules::EMPTY;
            let end = self.active.saturating_add(self.size_limit).min(self.widgets.len());
            let start = end.saturating_sub(self.size_limit);
            self.sized_range = start..end;
            debug_assert!(self.sized_range.contains(&self.active));
            for index in start..end {
                rules = rules.max(self.widgets[index].size_rules(size_mgr.re(), axis));
            }
            rules
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            self.align_hints = align;
            if let Some(child) = self.widgets.get_mut(self.active) {
                child.set_rect(mgr, rect, align);
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            // Latter condition is implied, but compiler doesn't know this:
            if self.sized_range.contains(&self.active) && self.active < self.widgets.len() {
                return self.widgets[self.active].find_id(coord);
            }
            None
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            if self.sized_range.contains(&self.active) && self.active < self.widgets.len() {
                draw.recurse(&mut self.widgets[self.active]);
            }
        }
    }

    impl Widget for Self {
        fn pre_configure(&mut self, _: &mut ConfigMgr, id: WidgetId) {
            self.core.id = id;
            self.id_map.clear();
        }

        fn spatial_nav(&mut self,
            _: &mut ConfigMgr,
            _: bool,
            from: Option<usize>,
        ) -> Option<usize> {
            match from {
                None => Some(self.active),
                Some(active) if active != self.active => Some(self.active),
                _ => None,
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

impl<W: Widget> Stack<W> {
    /// Construct a new, empty instance
    #[inline]
    pub fn new() -> Self {
        Self::new_vec(vec![])
    }

    /// Construct a new instance
    ///
    /// Initially, the first page (if any) will be shown. Use
    /// [`Self::with_active`] to change this.
    #[inline]
    pub fn new_vec(widgets: Vec<W>) -> Self {
        Stack {
            core: Default::default(),
            align_hints: Default::default(),
            widgets,
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
    /// value, [`TkAction::RECONFIGURE`]).
    #[inline]
    pub fn edit<F: FnOnce(&mut Vec<W>)>(&mut self, f: F) -> TkAction {
        f(&mut self.widgets);
        TkAction::RECONFIGURE
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
    ///     update mouse-cursor target ([`TkAction::REGION_MOVED`])
    /// -   Otherwise: resize the whole window ([`TkAction::RESIZE`])
    pub fn set_active(&mut self, mgr: &mut ConfigMgr, index: usize) {
        let old_index = self.active;
        self.active = index;
        if index >= self.widgets.len() {
            if old_index < self.widgets.len() {
                *mgr |= TkAction::REGION_MOVED;
            }
            return;
        }

        if self.sized_range.contains(&index) {
            if old_index != index {
                self.widgets[index].set_rect(mgr, self.core.rect, self.align_hints);
                *mgr |= TkAction::REGION_MOVED;
            }
        } else {
            *mgr |= TkAction::RESIZE;
        }
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
    /// This does not change the active page index.
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
    /// and then [`TkAction::RESIZE`] will be triggered.
    ///
    /// Returns the new page's index.
    pub fn push(&mut self, mgr: &mut ConfigMgr, widget: W) -> usize {
        let index = self.widgets.len();
        self.widgets.push(widget);
        let id = self.make_child_id(index);
        mgr.configure(id, &mut self.widgets[index]);
        if index == self.active {
            *mgr |= TkAction::RESIZE;
        }
        self.sized_range.end = self.sized_range.end.min(index);
        index
    }

    /// Remove the last child widget (if any) and return
    ///
    /// If this page was active then the previous page becomes active.
    pub fn pop(&mut self, mgr: &mut ConfigMgr) -> Option<W> {
        let result = self.widgets.pop();
        if let Some(w) = result.as_ref() {
            if self.active > 0 && self.active == self.widgets.len() {
                self.active -= 1;
                if self.sized_range.contains(&self.active) {
                    self.widgets[self.active].set_rect(mgr, self.core.rect, self.align_hints);
                } else {
                    *mgr |= TkAction::RESIZE;
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
    pub fn insert(&mut self, mgr: &mut ConfigMgr, index: usize, widget: W) {
        if self.active < index {
            self.sized_range.end = self.sized_range.end.min(index);
        } else {
            self.sized_range.start = (self.sized_range.start + 1).max(index + 1);
            self.sized_range.end += 1;
            self.active = self.active.saturating_add(1);
        }
        for v in self.id_map.values_mut() {
            if *v >= index {
                *v += 1;
            }
        }
        self.widgets.insert(index, widget);
        let id = self.make_child_id(index);
        mgr.configure(id, &mut self.widgets[index]);
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// If the active page is removed then the previous page (if any) becomes
    /// active.
    pub fn remove(&mut self, mgr: &mut ConfigMgr, index: usize) -> W {
        let w = self.widgets.remove(index);
        if w.id_ref().is_valid() {
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        if self.active == index {
            self.active = self.active.saturating_sub(1);
            if self.sized_range.contains(&self.active) {
                self.widgets[self.active].set_rect(mgr, self.core.rect, self.align_hints);
            } else {
                *mgr |= TkAction::RESIZE;
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
    /// then [`TkAction::RESIZE`] is triggered.
    pub fn replace(&mut self, mgr: &mut ConfigMgr, index: usize, mut w: W) -> W {
        std::mem::swap(&mut w, &mut self.widgets[index]);

        if w.id_ref().is_valid() {
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        let id = self.make_child_id(index);
        mgr.configure(id, &mut self.widgets[index]);

        if self.active < index {
            self.sized_range.end = self.sized_range.end.min(index);
        } else {
            self.sized_range.start = (self.sized_range.start + 1).max(index + 1);
            self.sized_range.end += 1;
            if index == self.active {
                *mgr |= TkAction::RESIZE;
            }
        }

        w
    }

    /// Append child widgets from an iterator
    ///
    /// New children are configured immediately. If a new page becomes active,
    /// then [`TkAction::RESIZE`] is triggered.
    pub fn extend<T: IntoIterator<Item = W>>(&mut self, mgr: &mut ConfigMgr, iter: T) {
        let old_len = self.widgets.len();
        self.widgets.extend(iter);
        for index in old_len..self.widgets.len() {
            let id = self.make_child_id(index);
            mgr.configure(id, &mut self.widgets[index]);
        }

        if (old_len..self.widgets.len()).contains(&self.active) {
            *mgr |= TkAction::RESIZE;
        }
    }

    /// Resize, using the given closure to construct new widgets
    ///
    /// New children are configured immediately. If a new page becomes active,
    /// then [`TkAction::RESIZE`] is triggered.
    pub fn resize_with<F: Fn(usize) -> W>(&mut self, mgr: &mut ConfigMgr, len: usize, f: F) {
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
                let mut widget = f(index);
                mgr.configure(id, &mut widget);
                self.widgets.push(widget);
            }

            if (old_len..len).contains(&self.active) {
                *mgr |= TkAction::RESIZE;
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
        Self::new_vec(iter.into_iter().collect())
    }
}
