// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A stack

use kas::layout::solve_size_rules;
use kas::prelude::*;
use std::collections::hash_map::{Entry, HashMap};
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

/// A stack of boxed widgets
///
/// This is a parametrisation of [`Stack`].
pub type BoxStack<Data> = Stack<Box<dyn Widget<Data = Data>>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    #[default]
    None,
    Configured,
    Sized,
}
impl State {
    fn is_configured(self) -> bool {
        self != State::None
    }
}

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
    /// By default, all pages are configured and sized. To avoid configuring
    /// hidden pages (thus preventing these pages from affecting size)
    /// call [`Self::set_size_limit`] or [`Self::with_size_limit`].
    #[impl_default]
    #[derive(Clone, Debug)]
    #[widget]
    pub struct Stack<W: Widget> {
        core: widget_core!(),
        align_hints: AlignHints,
        widgets: Vec<(W, State)>,
        active: usize,
        size_limit: usize = usize::MAX,
        next: usize,
        id_map: HashMap<usize, usize>, // map key of Id to index
    }

    impl Widget for Self {
        type Data = W::Data;

        fn for_child_node(
            &mut self,
            data: &W::Data,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some((w, _)) = self.widgets.get_mut(index) {
                closure(w.as_node(data));
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = SizeRules::EMPTY;
            for (index, entry) in self.widgets.iter_mut().enumerate() {
                if index < self.size_limit || index == self.active {
                    if entry.1.is_configured() {
                        rules = rules.max(entry.0.size_rules(sizer.re(), axis));
                        entry.1 = State::Sized;
                    } else {
                        entry.1 = State::None;
                    }
                } else {
                    // Ensure entry will be resized before becoming active
                    entry.1 = entry.1.min(State::Configured);
                }
            }
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.align_hints = hints;
            if let Some(entry) = self.widgets.get_mut(self.active) {
                debug_assert_eq!(entry.1, State::Sized);
                entry.0.set_rect(cx, rect, hints);
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            if let Some(entry) = self.widgets.get(self.active) {
                debug_assert_eq!(entry.1, State::Sized);
                entry.0.draw(draw.re());
            }
        }
    }

    impl Tile for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.widgets.get(index).map(|(w, _)| w.as_tile())
        }

        fn find_child_index(&self, id: &Id) -> Option<usize> {
            id.next_key_after(self.id_ref())
                .and_then(|k| self.id_map.get(&k).cloned())
        }

        fn nav_next(&self, _: bool, from: Option<usize>) -> Option<usize> {
            let active = match from {
                None => self.active,
                Some(active) if active != self.active => self.active,
                _ => return None,
            };
            if let Some(entry) = self.widgets.get(active) {
                debug_assert_eq!(entry.1, State::Sized);
                return Some(active);
            }
            None
        }

        fn probe(&self, coord: Coord) -> Id {
            if let Some(entry) = self.widgets.get(self.active) {
                debug_assert_eq!(entry.1, State::Sized);
                if let Some(id) = entry.0.try_probe(coord) {
                    return id;
                }
            }
            self.id()
        }
    }

    impl Events for Self {
        fn make_child_id(&mut self, index: usize) -> Id {
            if let Some((child, state)) = self.widgets.get(index) {
                if state.is_configured() {
                    debug_assert!(child.id_ref().is_valid());
                    if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                        self.id_map.insert(key, index);
                        return child.id();
                    }
                }

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

        fn configure(&mut self, _: &mut ConfigCx) {
            self.id_map.clear();
        }

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            for index in 0..self.widgets.len() {
                if index < self.size_limit || index == self.active {
                    let id = self.make_child_id(index);
                    let entry = &mut self.widgets[index];
                    cx.configure(entry.0.as_node(data), id);
                    if entry.1 == State::None {
                        entry.1 = State::Configured;
                    }
                } else {
                    // Ensure widget will be reconfigured before becoming active
                    self.widgets[index].1 = State::None;
                }
            }
        }

        fn update_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            if let Some((w, _)) = self.widgets.get_mut(self.active) {
                cx.update(w.as_node(data));
            }
        }
    }

    impl Index<usize> for Self {
        type Output = W;

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

impl<W: Widget> Stack<W> {
    /// Construct a new, empty instance
    ///
    /// See also [`Stack::from`].
    pub fn new() -> Self {
        Stack::default()
    }

    /// Limit the number of pages considered and sized
    ///
    /// By default, this is `usize::MAX`: all pages are configured and affect
    /// the stack's size requirements.
    ///
    /// Set this to 0 to avoid configuring all hidden pages.
    /// Set this to `n` to configure the active page *and* the first `n` pages.
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
            State::None
        );
        self.active = active;
        self
    }

    /// Set the active page
    pub fn set_active(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize) {
        let old_index = self.active;
        if old_index == index {
            return;
        }
        self.active = index;

        let rect = self.rect();
        let id = self.make_child_id(index);
        if let Some(entry) = self.widgets.get_mut(index) {
            let node = entry.0.as_node(data);

            if entry.1 == State::None {
                cx.configure(node, id);
                entry.1 = State::Configured;
            } else {
                cx.update(node);
            }

            if entry.1 == State::Configured {
                let Size(w, _h) = rect.size;
                // HACK: we should pass the known height here, but it causes
                // even distribution of excess space. Maybe SizeRules::solve_seq
                // should not always distribute excess space?
                solve_size_rules(&mut entry.0, cx.size_cx(), Some(w), None);
                entry.1 = State::Sized;
            }

            debug_assert_eq!(entry.1, State::Sized);
            entry.0.set_rect(cx, rect, self.align_hints);
            cx.region_moved();
        } else {
            if old_index < self.widgets.len() {
                cx.region_moved();
            }
        }
    }

    /// Get a direct reference to the active child widget, if any
    pub fn get_active(&self) -> Option<&W> {
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
    pub fn get(&self, index: usize) -> Option<&W> {
        self.widgets.get(index).map(|e| &e.0)
    }

    /// Returns a mutable reference to the page, if any
    pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
        self.widgets.get_mut(index).map(|e| &mut e.0)
    }

    /// Configure and size the page at index
    fn configure_and_size(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize) {
        let Size(w, h) = self.rect().size;
        let id = self.make_child_id(index);
        if let Some(entry) = self.widgets.get_mut(index) {
            cx.configure(entry.0.as_node(data), id);
            solve_size_rules(&mut entry.0, cx.size_cx(), Some(w), Some(h));
            entry.1 = State::Sized;
        }
    }

    /// Append a page
    ///
    /// The new page is not made active (the active index may be changed to
    /// avoid this). Consider calling [`Self::set_active`].
    ///
    /// Returns the new page's index.
    pub fn push(&mut self, cx: &mut ConfigCx, data: &W::Data, widget: W) -> usize {
        let index = self.widgets.len();
        if index == self.active {
            self.active = usize::MAX;
        }
        self.widgets.push((widget, State::None));

        if index < self.size_limit {
            self.configure_and_size(cx, data, index);
        }
        index
    }

    /// Remove the last child widget (if any) and return
    ///
    /// If this page was active then no page will be left active.
    /// Consider also calling [`Self::set_active`].
    pub fn pop(&mut self, cx: &mut EventState) -> Option<W> {
        let result = self.widgets.pop().map(|(w, _)| w);
        if let Some(w) = result.as_ref() {
            if self.active > 0 && self.active == self.widgets.len() {
                cx.region_moved();
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
    /// The active page does not change (the index of the active page may change instead).
    pub fn insert(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, widget: W) {
        if self.active >= index {
            self.active = self.active.saturating_add(1);
        }

        self.widgets.insert(index, (widget, State::None));

        for v in self.id_map.values_mut() {
            if *v >= index {
                *v += 1;
            }
        }

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
    pub fn remove(&mut self, cx: &mut EventState, index: usize) -> W {
        let (w, _) = self.widgets.remove(index);
        if w.id_ref().is_valid() {
            if let Some(key) = w.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        if self.active == index {
            self.active = usize::MAX;
            cx.region_moved();
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
    /// If the new child replaces the active page then [`Action::RESIZE`] is triggered.
    pub fn replace(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, mut widget: W) -> W {
        let entry = &mut self.widgets[index];
        std::mem::swap(&mut widget, &mut entry.0);
        entry.1 = State::None;

        if widget.id_ref().is_valid() {
            if let Some(key) = widget.id_ref().next_key_after(self.id_ref()) {
                self.id_map.remove(&key);
            }
        }

        if index < self.size_limit || index == self.active {
            self.configure_and_size(cx, data, index);
        }

        if index == self.active {
            cx.resize(self);
        }

        widget
    }

    /// Append child widgets from an iterator
    ///
    /// The new pages are not made active (the active index may be changed to
    /// avoid this). Consider calling [`Self::set_active`].
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
        for w in iter {
            let index = self.widgets.len();
            self.widgets.push((w, State::None));
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
    pub fn resize_with<F: Fn(usize) -> W>(
        &mut self,
        cx: &mut ConfigCx,
        data: &W::Data,
        len: usize,
        f: F,
    ) {
        let old_len = self.widgets.len();

        if len < old_len {
            loop {
                let (w, _) = self.widgets.pop().unwrap();
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
                self.widgets.push((f(index), State::None));
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

impl<W: Widget, I> From<I> for Stack<W>
where
    I: IntoIterator<Item = W>,
{
    #[inline]
    fn from(iter: I) -> Self {
        Self {
            widgets: iter.into_iter().map(|w| (w, State::None)).collect(),
            ..Default::default()
        }
    }
}
