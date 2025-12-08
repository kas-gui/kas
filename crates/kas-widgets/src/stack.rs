// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A stack

use kas::layout::solve_size_rules;
use kas::prelude::*;
use std::ops::{Index, IndexMut};

#[impl_self]
mod Page {
    /// A stack page (also known as a tab page)
    #[widget]
    #[layout(self.inner)]
    pub struct Page<A> {
        core: widget_core!(),
        #[widget]
        pub inner: Box<dyn Widget<Data = A>>,
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::TabPage
        }
    }

    impl Events for Self {
        type Data = A;
    }

    impl Self {
        /// Construct from a widget
        pub fn new(widget: impl Widget<Data = A> + 'static) -> Self {
            Page::new_boxed(Box::new(widget))
        }

        /// Construct from a boxed widget
        #[inline]
        pub fn new_boxed(inner: Box<dyn Widget<Data = A>>) -> Self {
            Page {
                core: Default::default(),
                inner,
            }
        }
    }
}

#[impl_self]
mod Stack {
    /// A stack of widgets
    ///
    /// A stack consists a set of child widgets, "pages", all of equal size.
    /// Only a single page is visible at a time. The page is "turned" by calling
    /// [`Self::set_active`].
    ///
    /// By default, all pages are configured and sized. To avoid configuring
    /// hidden pages (thus preventing these pages from affecting size)
    /// call [`Self::set_size_limit`] or [`Self::with_size_limit`].
    ///
    /// # Messages
    ///
    /// [`kas::messages::SetIndex`] may be used to change the page.
    #[widget]
    pub struct Stack<A> {
        core: widget_core!(),
        align_hints: AlignHints,
        // Page and key used in Id::make_child (if not usize::MAX)
        widgets: Vec<(Page<A>, usize)>,
        active: usize,
        size_limit: usize,
        next: usize,
    }

    impl Default for Self {
        fn default() -> Self {
            Stack {
                core: Default::default(),
                align_hints: AlignHints::NONE,
                widgets: Vec::new(),
                active: 0,
                size_limit: usize::MAX,
                next: 0,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let (mut min, mut ideal) = (0, 0);
            let mut m = (0, 0);
            let mut stretch = Stretch::None;

            for (i, entry) in self.widgets.iter_mut().enumerate() {
                if entry.1 != usize::MAX {
                    let rules = entry.0.size_rules(cx, axis);
                    ideal = ideal.max(rules.ideal_size());
                    m = (m.0.max(rules.margins().0), m.1.max(rules.margins().1));
                    stretch = stretch.max(rules.stretch());
                    if i == self.active {
                        min = rules.min_size();
                    }
                }
            }

            SizeRules::new(min, ideal, stretch).with_margins(m)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.align_hints = hints;

            for entry in self.widgets.iter_mut() {
                if entry.1 != usize::MAX {
                    entry.0.set_rect(cx, rect, hints);
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            if let Some(entry) = self.widgets.get(self.active) {
                debug_assert!(entry.1 != usize::MAX);
                entry.0.draw(draw.re());
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::None
        }

        #[inline]
        fn child_indices(&self) -> ChildIndices {
            if self.active < self.widgets.len() {
                ChildIndices::one(self.active)
            } else {
                ChildIndices::none()
            }
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.widgets
                .get(index)
                .filter(|w| w.1 != usize::MAX)
                .map(|w| w.0.as_tile())
        }

        fn find_child_index(&self, id: &Id) -> Option<usize> {
            // NOTE: this approach is O(n) where n = number of pages. Since a
            // Stack should have a small number of pages this is acceptable.

            let key = id.next_key_after(self.id_ref())?;
            for (i, w) in self.widgets.iter().enumerate() {
                if w.1 == key {
                    return Some(i);
                }
            }
            None
        }

        fn nav_next(&self, _: bool, from: Option<usize>) -> Option<usize> {
            let active = match from {
                None => self.active,
                Some(active) if active != self.active => self.active,
                _ => return None,
            };
            if let Some(entry) = self.widgets.get(active) {
                debug_assert!(entry.1 != usize::MAX);
                return Some(active);
            }
            None
        }
    }

    impl Events for Self {
        fn make_child_id(&mut self, index: usize) -> Id {
            let Some((child, key)) = self.widgets.get(index) else {
                return Id::default();
            };
            let id = child.id_ref();
            if id.is_valid()
                && let Some(k) = id.next_key_after(self.id_ref())
                && (*key == k || self.widgets.iter().all(|entry| k != entry.1))
            {
                let id = id.clone();
                self.widgets[index].1 = k;
                return id;
            }

            loop {
                let key = self.next;
                self.next += 1;
                if self.widgets.iter().any(|entry| entry.1 == key) {
                    continue;
                }

                self.widgets[index].1 = key;
                return self.id_ref().make_child(key);
            }
        }

        fn probe(&self, coord: Coord) -> Id {
            if let Some(entry) = self.widgets.get(self.active) {
                debug_assert!(entry.1 != usize::MAX);
                if let Some(id) = entry.0.try_probe(coord) {
                    return id;
                }
            }
            self.id()
        }

        fn configure(&mut self, _: &mut ConfigCx) {
            for entry in self.widgets.iter_mut() {
                entry.1 = usize::MAX;
            }
        }

        #[inline]
        fn recurse_indices(&self) -> ChildIndices {
            let end = self.widgets.len().min(self.size_limit.max(self.active + 1));
            let start = end.saturating_sub(self.size_limit).min(self.active);
            ChildIndices::range(start..end)
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &A) {
            if let Some(kas::messages::SetIndex(index)) = cx.try_pop() {
                self.set_active(cx, data, index);
            }
        }
    }

    impl Widget for Self {
        type Data = A;

        fn child_node<'n>(&'n mut self, data: &'n A, index: usize) -> Option<Node<'n>> {
            self.widgets
                .get_mut(index)
                .filter(|w| w.1 != usize::MAX)
                .map(|w| w.0.as_node(data))
        }
    }

    impl Index<usize> for Self {
        type Output = Page<A>;

        fn index(&self, index: usize) -> &Self::Output {
            &self.widgets[index].0
        }
    }

    impl IndexMut<usize> for Self {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            &mut self.widgets[index].0
        }
    }
}

impl<A> Stack<A> {
    /// Construct a new, empty instance
    ///
    /// See also [`Stack::from`].
    pub fn new() -> Self {
        Stack::default()
    }

    /// Limit the number of pages configured and sized
    ///
    /// By default, this is `usize::MAX`: all pages are configured and affect
    /// the stack's size requirements.
    ///
    /// Set this to 0 to avoid configuring all hidden pages.
    /// Set this to `n` to configure the active page *and* the first `n` pages.
    ///
    /// This affects configuration, sizing and message handling for inactive pages.
    pub fn set_size_limit(&mut self, limit: usize) {
        self.size_limit = limit;
    }

    /// Limit the number of pages configured and sized (inline)
    ///
    /// By default, this is `usize::MAX`: all pages are configured and affect
    /// the stack's size requirements.
    ///
    /// Set this to 0 to avoid configuring all hidden pages.
    /// Set this to `n` to configure the active page *and* the first `n` pages.
    pub fn with_size_limit(mut self, limit: usize) -> Self {
        self.size_limit = limit;
        self
    }

    /// Get the index of the active widget
    #[inline]
    pub fn active(&self) -> usize {
        self.active
    }

    /// Set the active widget (inline)
    ///
    /// Unlike [`Self::set_active`], this does not update anything; it is
    /// assumed that this method is only used before the UI is run.
    #[inline]
    pub fn with_active(mut self, active: usize) -> Self {
        debug_assert_eq!(
            self.widgets
                .get(self.active)
                .map(|e| e.1)
                .unwrap_or_default(),
            usize::MAX
        );
        self.active = active;
        self
    }

    /// Change the active page and update
    ///
    /// If `index == self.active()` then nothing happens.
    /// If `index >= self.len()` then nothing will be displayed.
    /// If the page is changed successfully, the newly active page is updated.
    pub fn set_active(&mut self, cx: &mut ConfigCx, data: &A, index: usize) {
        let old_index = self.active;
        if old_index == index {
            return;
        }
        self.active = index;

        let rect = self.rect();
        if index < self.widgets.len() {
            let id = (self.widgets[index].1 == usize::MAX).then_some(self.make_child_id(index));
            let entry = &mut self.widgets[index];
            let node = entry.0.as_node(data);
            if let Some(id) = id {
                cx.configure(node, id);
                debug_assert!(entry.1 != usize::MAX);
            } else {
                cx.update(node);
            }

            let Size(w, _h) = rect.size;
            // HACK: we should pass the known height here, but it causes
            // even distribution of excess space. Maybe SizeRules::solve_seq
            // should not always distribute excess space?
            solve_size_rules(&mut entry.0, &mut cx.size_cx(), Some(w), None);

            entry.0.set_rect(&mut cx.size_cx(), rect, self.align_hints);
            cx.region_moved();
        } else {
            if old_index < self.widgets.len() {
                cx.region_moved();
            }
        }
    }

    /// Get a direct reference to the active child page, if any
    pub fn get_active(&self) -> Option<&Page<A>> {
        if self.active < self.widgets.len() {
            Some(&self.widgets[self.active].0)
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
    }

    /// Returns a reference to the page, if any
    pub fn get(&self, index: usize) -> Option<&Page<A>> {
        self.widgets.get(index).map(|e| &e.0)
    }

    /// Returns a mutable reference to the page, if any
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Page<A>> {
        self.widgets.get_mut(index).map(|e| &mut e.0)
    }

    /// Configure and size the page at index
    fn configure_and_size(&mut self, cx: &mut ConfigCx, data: &A, index: usize) {
        let Size(w, h) = self.rect().size;
        let id = self.make_child_id(index);
        if let Some(entry) = self.widgets.get_mut(index) {
            cx.configure(entry.0.as_node(data), id);
            solve_size_rules(&mut entry.0, &mut cx.size_cx(), Some(w), Some(h));
            debug_assert!(entry.1 != usize::MAX);
        }
    }

    /// Append a page
    ///
    /// The new page is not made active (the active index may be changed to
    /// avoid this). Consider calling [`Self::set_active`].
    ///
    /// Returns the new page's index.
    pub fn push(&mut self, cx: &mut ConfigCx, data: &A, page: Page<A>) -> usize {
        let index = self.widgets.len();
        if index == self.active {
            self.active = usize::MAX;
        }
        self.widgets.push((page, usize::MAX));

        if index < self.size_limit {
            self.configure_and_size(cx, data, index);
        }
        index
    }

    /// Remove the last child widget (if any) and return
    ///
    /// If this page was active then no page will be left active.
    /// Consider also calling [`Self::set_active`].
    pub fn pop(&mut self, cx: &mut EventState) -> Option<Page<A>> {
        let result = self.widgets.pop().map(|w| w.0);
        if result.is_some() {
            if self.active > 0 && self.active == self.widgets.len() {
                cx.region_moved();
            }
        }
        result
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// The active page does not change (the index of the active page may change instead).
    pub fn insert(&mut self, cx: &mut ConfigCx, data: &A, index: usize, page: Page<A>) {
        if self.active >= index {
            self.active = self.active.saturating_add(1);
        }

        self.widgets.insert(index, (page, usize::MAX));

        if index < self.size_limit {
            self.configure_and_size(cx, data, index);
        }
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// If this page was active then no page will be left active.
    /// Consider also calling [`Self::set_active`].
    pub fn remove(&mut self, cx: &mut EventState, index: usize) -> Page<A> {
        let w = self.widgets.remove(index);

        if self.active == index {
            self.active = usize::MAX;
            cx.region_moved();
        }

        for entry in self.widgets[index..].iter_mut() {
            entry.1 -= 1;
        }
        w.0
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// If the new child replaces the active page then a resize is triggered.
    pub fn replace(
        &mut self,
        cx: &mut ConfigCx,
        data: &A,
        index: usize,
        mut page: Page<A>,
    ) -> Page<A> {
        let entry = &mut self.widgets[index];
        std::mem::swap(&mut page, &mut entry.0);
        entry.1 = usize::MAX;

        if index < self.size_limit || index == self.active {
            self.configure_and_size(cx, data, index);
        }

        if index == self.active {
            cx.resize();
        }

        page
    }

    /// Append child widgets from an iterator
    ///
    /// The new pages are not made active (the active index may be changed to
    /// avoid this). Consider calling [`Self::set_active`].
    pub fn extend<T: IntoIterator<Item = Page<A>>>(
        &mut self,
        cx: &mut ConfigCx,
        data: &A,
        iter: T,
    ) {
        let old_len = self.widgets.len();
        let iter = iter.into_iter();
        if let Some(ub) = iter.size_hint().1 {
            self.widgets.reserve(ub);
        }
        for w in iter {
            let index = self.widgets.len();
            self.widgets.push((w, usize::MAX));
            if index < self.size_limit {
                self.configure_and_size(cx, data, index);
            }
        }

        if (old_len..self.widgets.len()).contains(&self.active) {
            self.active = usize::MAX;
        }
    }

    /// Resize, using the given closure to construct new widgets
    ///
    /// The new pages are not made active (the active index may be changed to
    /// avoid this). Consider calling [`Self::set_active`].
    pub fn resize_with<F: Fn(usize) -> Page<A>>(
        &mut self,
        cx: &mut ConfigCx,
        data: &A,
        len: usize,
        f: F,
    ) {
        let old_len = self.widgets.len();

        if len < old_len {
            loop {
                let _ = self.widgets.pop().unwrap();
                if len == self.widgets.len() {
                    break;
                }
            }
        }

        if len > old_len {
            self.widgets.reserve(len - old_len);
            for index in old_len..len {
                self.widgets.push((f(index), usize::MAX));
                if index < self.size_limit {
                    self.configure_and_size(cx, data, index);
                }
            }

            if (old_len..len).contains(&self.active) {
                self.active = usize::MAX;
            }
        }
    }
}

impl<A, I> From<I> for Stack<A>
where
    I: IntoIterator<Item = Page<A>>,
{
    #[inline]
    fn from(iter: I) -> Self {
        Self {
            widgets: iter.into_iter().map(|w| (w, usize::MAX)).collect(),
            ..Default::default()
        }
    }
}
