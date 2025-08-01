// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A tabbed stack

use crate::adapt::{AdaptEvents, AdaptWidget};
use crate::{AccessLabel, Row, Stack};
use kas::messages::{Select, SetIndex};
use kas::prelude::*;
use kas::theme::FrameStyle;

#[impl_self]
mod Tab {
    /// A tab
    ///
    /// This is a special variant of `Button` which sends a [`Select`] on press.
    ///
    /// # Messages
    ///
    /// [`kas::messages::Activate`] may be used to open this tab.
    #[widget]
    #[layout(frame!(self.label).with_style(FrameStyle::Tab))]
    pub struct Tab {
        core: widget_core!(),
        #[widget]
        label: AccessLabel,
    }

    impl Self {
        /// Construct a button with given `label` widget
        #[inline]
        pub fn new(label: impl Into<AccessString>) -> Self {
            Tab {
                core: Default::default(),
                label: AccessLabel::new(label),
            }
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            cx.set_label(self.label.id());
            Role::Tab
        }

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = ();

        fn handle_event(&mut self, cx: &mut EventCx, _: &(), event: Event) -> IsUsed {
            event.on_activate(cx, self.id(), |cx| {
                cx.push(Select);
                Used
            })
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &()) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                cx.push(Select);
                cx.depress_with_key(self.id(), code);
            }
        }
    }

    impl<T: Into<AccessString>> From<T> for Tab {
        fn from(label: T) -> Self {
            Tab::new(label)
        }
    }
}

/// A tabbed stack of boxed widgets
///
/// This is a parametrisation of [`TabStack`].
pub type BoxTabStack<Data> = TabStack<Box<dyn Widget<Data = Data>>>;

#[impl_self]
mod TabStack {
    /// A tabbed stack of widgets
    ///
    /// A stack consists a set of child widgets, "pages", all of equal size.
    /// Only a single page is visible at a time. The page is "turned" via tab
    /// handles or calling [`Self::set_active`].
    ///
    /// Type parameter `D` controls the position of tabs relative to the stack;
    /// default value is [`Direction::Up`]: tabs are positioned above the stack.
    /// Within the bar, items are always left-to-right
    /// (TODO: support for vertical bars).
    ///
    /// This may only be parametrised with a single widget type, thus usually
    /// it will be necessary to box children (this is what [`BoxTabStack`] is).
    ///
    /// See also the main implementing widget: [`Stack`].
    ///
    /// # Messages
    ///
    /// [`kas::messages::SetIndex`] may be used to change the page.
    #[impl_default(Self::new())]
    #[widget]
    #[layout(list![self.stack, self.tabs].with_direction(self.direction))]
    pub struct TabStack<W: Widget> {
        core: widget_core!(),
        direction: Direction,
        #[widget(&())]
        tabs: AdaptEvents<Row<Vec<Tab>>>, // TODO: want a TabBar widget for scrolling support?
        #[widget]
        stack: Stack<W>,
        on_change: Option<Box<dyn Fn(&mut EventCx, &W::Data, usize, &str)>>,
    }

    impl Self {
        /// Construct a new, empty instance
        ///
        /// See also [`TabStack::from`].
        pub fn new() -> Self {
            Self {
                core: Default::default(),
                direction: Direction::Up,
                stack: Stack::new(),
                tabs: Row::new(vec![]).map_message(|index, Select| SetIndex(index)),
                on_change: None,
            }
        }

        /// Set the position of tabs relative to content
        ///
        /// Default value: [`Direction::Up`]
        pub fn set_direction(&mut self, cx: &mut EventState, direction: Direction) {
            if direction == self.direction {
                return;
            }

            self.direction = direction;
            // Note: most of the time Action::SET_RECT would be enough, but margins can be different
            cx.resize(self);
        }

        /// Call the handler `f` on page change
        ///
        /// `f` receives as parameters input data, page index and tab title.
        #[inline]
        #[must_use]
        pub fn with(mut self, f: impl Fn(&mut EventCx, &W::Data, usize, &str) + 'static) -> Self {
            debug_assert!(self.on_change.is_none());
            self.on_change = Some(Box::new(f));
            self
        }

        /// Send the message generated by `f` on page change
        ///
        /// `f` receives as page index and tab title.
        #[inline]
        #[must_use]
        pub fn with_msg<M>(self, f: impl Fn(usize, &str) -> M + 'static) -> Self
        where
            M: std::fmt::Debug + 'static,
        {
            self.with(move |cx, _, index, title| cx.push(f(index, title)))
        }
    }

    impl Tile for Self {
        fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
            let reverse = reverse ^ !self.direction.is_reversed();
            kas::util::nav_next(reverse, from, self.child_indices())
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn handle_messages(&mut self, cx: &mut EventCx, data: &W::Data) {
            if let Some(SetIndex(index)) = cx.try_pop() {
                self.set_active(&mut cx.config_cx(), data, index);
                if let Some(ref f) = self.on_change {
                    let title = self.tabs.inner[index].as_str();
                    f(cx, data, index, title);
                }
            }
        }
    }
}

impl<W: Widget> TabStack<W> {
    /// Limit the number of pages considered and sized
    ///
    /// By default, this is `usize::MAX`: all pages are configured and affect
    /// the stack's size requirements.
    ///
    /// Set this to 0 to avoid configuring all hidden pages.
    /// Set this to `n` to configure the active page *and* the first `n` pages.
    pub fn set_size_limit(&mut self, limit: usize) {
        self.stack.set_size_limit(limit);
    }

    /// Limit the number of pages configured and sized (inline)
    ///
    /// By default, this is `usize::MAX`: all pages are configured and affect
    /// the stack's size requirements.
    ///
    /// Set this to 0 to avoid configuring all hidden pages.
    /// Set this to `n` to configure the active page *and* the first `n` pages.
    pub fn with_size_limit(mut self, limit: usize) -> Self {
        self.stack.set_size_limit(limit);
        self
    }

    /// Get the index of the active page
    #[inline]
    pub fn active(&self) -> usize {
        self.stack.active()
    }

    /// Set the active page (inline)
    ///
    /// Unlike [`Self::set_active`], this does not update anything; it is
    /// assumed that sizing happens afterwards.
    #[inline]
    pub fn with_active(mut self, active: usize) -> Self {
        self.stack = self.stack.with_active(active);
        self
    }

    /// Set the active page
    pub fn set_active(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize) {
        self.stack.set_active(cx, data, index);
    }

    /// Get a direct reference to the active child widget, if any
    pub fn get_active(&self) -> Option<&W> {
        self.stack.get_active()
    }

    /// True if there are no pages
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Returns the number of pages
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Remove all pages
    ///
    /// This does not change the active page index.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.tabs.inner.clear();
    }

    /// Get a page
    pub fn get(&self, index: usize) -> Option<&W> {
        self.stack.get(index)
    }

    /// Get a page
    pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
        self.stack.get_mut(index)
    }

    /// Get a tab
    pub fn get_tab(&self, index: usize) -> Option<&Tab> {
        self.tabs.inner.get(index)
    }

    /// Get a tab
    pub fn get_tab_mut(&mut self, index: usize) -> Option<&mut Tab> {
        self.tabs.inner.get_mut(index)
    }

    /// Append a page
    ///
    /// The new page is not made active (the active index may be changed to
    /// avoid this). Consider calling [`Self::set_active`].
    ///
    /// Returns the new page's index.
    pub fn push(&mut self, cx: &mut ConfigCx, data: &W::Data, tab: Tab, widget: W) -> usize {
        let ti = self.tabs.inner.push(cx, &(), tab);
        let si = self.stack.push(cx, data, widget);
        debug_assert_eq!(ti, si);
        si
    }

    /// Remove the last child widget (if any) and return
    ///
    /// If this page was active then no page will be left active.
    /// Consider also calling [`Self::set_active`].
    pub fn pop(&mut self, cx: &mut EventState) -> Option<(Tab, W)> {
        let tab = self.tabs.inner.pop(cx);
        let w = self.stack.pop(cx);
        debug_assert_eq!(tab.is_some(), w.is_some());
        tab.zip(w)
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// The active page does not change (the index of the active page may change instead).
    pub fn insert(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, tab: Tab, widget: W) {
        self.tabs.inner.insert(cx, &(), index, tab);
        self.stack.insert(cx, data, index, widget);
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// If this page was active then no page will be left active.
    /// Consider also calling [`Self::set_active`].
    pub fn remove(&mut self, cx: &mut EventState, index: usize) -> (Tab, W) {
        let tab = self.tabs.inner.remove(cx, index);
        let stack = self.stack.remove(cx, index);
        (tab, stack)
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// If the new child replaces the active page then [`Action::RESIZE`] is triggered.
    pub fn replace(&mut self, cx: &mut ConfigCx, data: &W::Data, index: usize, w: W) -> W {
        self.stack.replace(cx, data, index, w)
    }

    /// Append child widgets from an iterator
    ///
    /// The new pages are not made active (the active index may be changed to
    /// avoid this). Consider calling [`Self::set_active`].
    pub fn extend<T: IntoIterator<Item = (Tab, W)>>(
        &mut self,
        cx: &mut ConfigCx,
        data: &W::Data,
        iter: T,
    ) {
        let iter = iter.into_iter();
        // let min_len = iter.size_hint().0;
        // self.tabs.reserve(min_len);
        // self.stack.reserve(min_len);
        for (tab, w) in iter {
            self.tabs.inner.push(cx, &(), tab);
            self.stack.push(cx, data, w);
        }
    }
}

impl<W: Widget, T, I> From<I> for TabStack<W>
where
    Tab: From<T>,
    I: IntoIterator<Item = (T, W)>,
{
    #[inline]
    fn from(iter: I) -> Self {
        let iter = iter.into_iter();
        let min_len = iter.size_hint().0;
        let mut stack = Vec::with_capacity(min_len);
        let mut tabs = Vec::with_capacity(min_len);
        for (tab, w) in iter {
            stack.push(w);
            tabs.push(Tab::from(tab));
        }
        Self {
            stack: Stack::from(stack),
            tabs: Row::new(tabs).map_message(|index, Select| SetIndex(index)),
            ..Default::default()
        }
    }
}
