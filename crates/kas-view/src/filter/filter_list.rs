// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter-list adapter

use super::Filter;
use crate::{DataClerk, Driver, ListView};
use kas::dir::{Direction, Directional};
use kas::event::{EventCx, EventState};
use kas::{autoimpl, impl_self, Events, Widget};
use kas_widgets::edit::{EditBox, EditField, EditGuard};
use kas_widgets::{ScrollBarMode, ScrollBars};
use std::fmt::Debug;

#[derive(Debug, Default)]
pub struct SetFilter<T: Debug>(pub T);

/// An [`EditGuard`] which sends a [`SetFilter`] message on every change
///
/// This may be used for search-as-you-type.
pub struct KeystrokeGuard;
impl EditGuard for KeystrokeGuard {
    type Data = ();

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Self::Data) {
        cx.push(SetFilter(edit.as_str().to_string()));
    }
}

/// An [`EditGuard`] which sends a [`SetFilter`] message on activate and focus loss
///
/// This may be used for search-as-you-type.
pub struct AflGuard;
impl EditGuard for AflGuard {
    type Data = ();

    #[inline]
    fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Self::Data) {
        cx.push(SetFilter(edit.as_str().to_string()));
    }
}

#[impl_self]
mod FilterBoxListView {
    /// An [`EditBox`] above a filtered [`ListView`]
    ///
    /// This is essentially just two widgets with "glue" to handle a
    /// [`SetFilter`] message from the [`EditBox`].
    #[autoimpl(Scrollable using self.list)]
    #[widget]
    #[layout(column! [self.edit, self.list])]
    pub struct FilterBoxListView<F, A, V, G = KeystrokeGuard, D = Direction>
    where
        F: Filter<A::Item, Value = String>,
        A: DataClerk<usize, Data = F>,
        V: Driver<A::Key, A::Item>,
        G: EditGuard<Data = ()>,
        D: Directional,
    {
        core: widget_core!(),
        filter: F,
        #[widget(&())]
        edit: EditBox<G>,
        #[widget(&self.filter)]
        list: ScrollBars<ListView<A, V, D>>,
    }

    impl Self {
        /// Construct
        ///
        /// Parameter `guard` may be [`KeystrokeGuard`], [`AflGuard`] or a
        /// custom implementation.
        pub fn new(filter: F, list: ListView<A, V, D>, guard: G) -> Self {
            Self {
                core: Default::default(),
                filter,
                edit: EditBox::new(guard),
                list: ScrollBars::new(list),
            }
        }

        /// Set fixed visibility of scroll bars (inline)
        #[inline]
        pub fn with_fixed_bars(mut self, horiz: bool, vert: bool) -> Self
        where
            Self: Sized,
        {
            self.list = self.list.with_fixed_bars(horiz, vert);
            self
        }

        /// Set fixed, invisible bars (inline)
        ///
        /// In this mode scroll bars are either enabled but invisible until
        /// hovered by the mouse or disabled completely.
        #[inline]
        pub fn with_invisible_bars(mut self, horiz: bool, vert: bool) -> Self
        where
            Self: Sized,
        {
            self.list = self.list.with_invisible_bars(horiz, vert);
            self
        }

        /// Get current mode of scroll bars
        #[inline]
        pub fn scroll_bar_mode(&self) -> ScrollBarMode {
            self.list.scroll_bar_mode()
        }

        /// Set scroll bar mode
        pub fn set_scroll_bar_mode(&mut self, cx: &mut EventState, mode: ScrollBarMode) {
            self.list.set_scroll_bar_mode(cx, mode);
        }

        /// Access the inner list widget
        #[inline]
        pub fn list(&self) -> &ListView<A, V, D> {
            self.list.inner()
        }

        /// Access the inner list widget mutably
        #[inline]
        pub fn list_mut(&mut self) -> &mut ListView<A, V, D> {
            self.list.inner_mut()
        }
    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, data: &()) {
            if let Some(SetFilter(value)) = cx.try_pop() {
                self.filter.set_filter(value);
                cx.update(self.as_node(data));
            }
        }
    }
}
