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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    #[autoimpl(Default)]
    #[derive(Clone, Debug)]
    #[widget]
    pub struct Stack<W: Widget> {
        core: widget_core!(),
        widgets: Vec<(W, State)>,
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
            if let Some((w, _)) = self.widgets.get_mut(index) {
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
            self.widgets.get(index).map(|(w, _)| w.as_layout())
        }

        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            id.next_key_after(self.id_ref())
                .and_then(|k| self.id_map.get(&k).cloned())
        }

        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = SizeRules::EMPTY;
            let mut index = 0;
            let end = self.widgets.len().min(self.size_limit);
            loop {
                if index == end {
                    if self.active >= end {
                        index = self.active;
                    } else {
                        break rules;
                    }
                } else if index > end {
                    break rules;
                }

                if let Some(entry) = self.widgets.get_mut(index) {
                    if entry.1.is_configured() {
                        rules = rules.max(entry.0.size_rules(sizer.re(), axis));
                        entry.1 = State::Sized;
                    }
                }

                index += 1;
            }
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            if let Some(entry) = self.widgets.get_mut(self.active) {
                debug_assert_eq!(entry.1, State::Sized);
                entry.0.set_rect(cx, rect);
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
            if let Some(entry) = self.widgets.get_mut(self.active) {
                debug_assert_eq!(entry.1, State::Sized);
                return entry.0.find_id(coord);
            }
            None
        }

        fn draw(&mut self, mut draw: DrawCx) {
            if let Some(entry) = self.widgets.get_mut(self.active) {
                debug_assert_eq!(entry.1, State::Sized);
                draw.recurse(&mut entry.0);
            }
        }
    }

    impl Events for Self {
        fn make_child_id(&mut self, index: usize) -> WidgetId {
            if let Some((child, state)) = self.widgets.get(index) {
                if state.is_configured() {
                    debug_assert!(child.id_ref().is_valid());
                    if let Some(key) = child.id_ref().next_key_after(self.id_ref()) {
                        debug_assert_eq!(self.id_map.get(&key), Some(&index));
                    } else {
                        debug_assert!(false);
                    }
                    return child.id();
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
            let mut index = 0;
            let end = self.widgets.len().min(self.size_limit);
            loop {
                if index == end {
                    if self.active >= end {
                        index = self.active;
                    } else {
                        break;
                    }
                } else if index > end {
                    break;
                }

                let id = self.make_child_id(index);
                if let Some(entry) = self.widgets.get_mut(index) {
                    cx.configure(entry.0.as_node(data), id);
                    if entry.1 == State::None {
                        entry.1 = State::Configured;
                    }
                }

                index += 1;
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
    /// assumed that sizing happens afterwards.
    #[inline]
    pub fn with_active(mut self, active: usize) -> Self {
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
                *cx |= Action::RESIZE;
            } else {
                debug_assert_eq!(entry.1, State::Sized);
                entry.0.set_rect(cx, self.core.rect);
                *cx |= Action::REGION_MOVED;
            }
        } else {
            if old_index < self.widgets.len() {
                *cx |= Action::REGION_MOVED;
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
        let id = self.make_child_id(index);
        if let Some(entry) = self.widgets.get_mut(index) {
            cx.configure(entry.0.as_node(data), id);
            let Size(w, h) = self.core.rect.size;
            solve_size_rules(&mut entry.0, cx.size_cx(), Some(w), Some(h), None, None);
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
                *cx |= Action::REGION_MOVED;
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
            *cx |= Action::REGION_MOVED;
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
            *cx |= Action::RESIZE;
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
