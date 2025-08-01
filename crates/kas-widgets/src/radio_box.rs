// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccessLabel;
use kas::prelude::*;
use kas::theme::Feature;
use std::fmt::Debug;
use std::time::Instant;

#[impl_self]
mod RadioBox {
    /// A bare radio box (no label)
    ///
    /// See also [`RadioButton`] which includes a label.
    ///
    /// # Messages
    ///
    /// [`kas::messages::Activate`] may be used to select this radio button.
    #[autoimpl(Debug ignore self.state_fn, self.on_select)]
    #[widget]
    pub struct RadioBox<A> {
        core: widget_core!(),
        state: bool,
        last_change: Option<Instant>,
        state_fn: Box<dyn Fn(&ConfigCx, &A) -> bool>,
        on_select: Option<Box<dyn Fn(&mut EventCx, &A)>>,
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = A;

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            let new_state = (self.state_fn)(cx, data);
            if self.state != new_state {
                self.state = new_state;
                self.last_change = Some(Instant::now());
                cx.redraw(self);
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &Self::Data, event: Event) -> IsUsed {
            event.on_activate(cx, self.id(), |cx| {
                self.select(cx, data);
                Used
            })
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                self.select(cx, data);
                cx.depress_with_key(self.id(), code);
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            sizer.feature(Feature::RadioBox, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = hints.complete_center();
            let rect = cx.align_feature(Feature::RadioBox, rect, align);
            widget_set_rect!(rect);
        }

        fn draw(&self, mut draw: DrawCx) {
            draw.radio_box(self.rect(), self.state, self.last_change);
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::RadioButton(self.state)
        }
    }

    impl Self {
        /// Construct a radio box
        ///
        /// - `state_fn` extracts the current state from input data
        #[inline]
        pub fn new(state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static) -> Self {
            RadioBox {
                core: Default::default(),
                state: false,
                last_change: None,
                state_fn: Box::new(state_fn),
                on_select: None,
            }
        }

        /// Call the handler `f` on selection
        ///
        /// No handler is called on deselection.
        #[inline]
        #[must_use]
        pub fn with(mut self, f: impl Fn(&mut EventCx, &A) + 'static) -> Self {
            debug_assert!(self.on_select.is_none());
            self.on_select = Some(Box::new(f));
            self
        }

        /// Construct a radio box
        ///
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `msg_fn` is emitted when selected
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
            msg_fn: impl Fn() -> M + 'static,
        ) -> Self {
            RadioBox::new(state_fn).with(move |cx, _| cx.push(msg_fn()))
        }

        /// Construct a radio box
        ///
        /// This radio box expects data of type `A` and will appear set when
        /// input `data == value`. Additionally, on selection, it will emit a
        /// copy of `value` as a message.
        #[inline]
        pub fn new_value(value: A) -> Self
        where
            A: Clone + Debug + Eq + 'static,
        {
            let v2 = value.clone();
            Self::new(move |_, data| *data == value).with(move |cx, _| cx.push(v2.clone()))
        }

        fn select(&mut self, cx: &mut EventCx, data: &A) {
            self.state = true;
            if let Some(ref f) = self.on_select {
                f(cx, data);
            }

            self.last_change = Some(Instant::now());
            cx.redraw(self);
        }
    }
}

#[impl_self]
mod RadioButton {
    /// A radio button with label
    ///
    /// See also [`RadioBox`] which excludes the label.
    ///
    /// # Messages
    ///
    /// [`kas::messages::Activate`] may be used to select this radio button.
    #[widget]
    #[layout(list![self.inner, self.label].with_direction(self.direction()))]
    pub struct RadioButton<A> {
        core: widget_core!(),
        #[widget]
        inner: RadioBox<A>,
        #[widget(&())]
        label: AccessLabel,
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            kas::MacroDefinedLayout::set_rect(self, cx, rect, hints);
            let dir = self.direction();
            crate::check_box::shrink_to_text(&mut self.rect(), dir, &self.label);
        }
    }

    impl Tile for Self {
        fn role_child_properties(&self, cx: &mut dyn RoleCx, index: usize) {
            if index == widget_index!(self.inner) {
                cx.set_label(self.label.id());
            }
        }

        fn nav_next(&self, _: bool, from: Option<usize>) -> Option<usize> {
            from.xor(Some(widget_index!(self.inner)))
        }

        fn probe(&self, _: Coord) -> Id {
            self.inner.id()
        }
    }

    impl Events for Self {
        type Data = A;

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.make_child_id(widget_index!(self.inner));
            if id.is_valid() {
                cx.configure(self.inner.as_node(data), id);

                if let Some(key) = self.label.access_key() {
                    cx.add_access_key(self.inner.id_ref(), key.clone());
                }
            }

            let id = self.make_child_id(widget_index!(self.label));
            if id.is_valid() {
                cx.configure(self.label.as_node(&()), id);
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                self.inner.select(cx, data);
                cx.depress_with_key(self.inner.id(), code);
            }
        }
    }

    impl Self {
        /// Construct a radio button with the given `label`
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        #[inline]
        pub fn new(
            label: impl Into<AccessString>,
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
        ) -> Self {
            RadioButton {
                core: Default::default(),
                inner: RadioBox::new(state_fn),
                label: AccessLabel::new(label.into()),
            }
        }

        /// Call the handler `f` on selection
        ///
        /// No handler is called on deselection.
        #[inline]
        #[must_use]
        pub fn with(self, f: impl Fn(&mut EventCx, &A) + 'static) -> Self {
            RadioButton {
                core: self.core,
                inner: self.inner.with(f),
                label: self.label,
            }
        }

        /// Construct a radio button
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `msg_fn` is emitted when selected
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            label: impl Into<AccessString>,
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
            msg_fn: impl Fn() -> M + 'static,
        ) -> Self {
            RadioButton::new(label, state_fn).with(move |cx, _| cx.push(msg_fn()))
        }

        /// Construct a radio button
        ///
        /// This radio button expects data of type `A` and will appear set when
        /// input `data == value`. Additionally, on selection, it will emit a
        /// copy of `value` as a message.
        #[inline]
        pub fn new_value(label: impl Into<AccessString>, value: A) -> Self
        where
            A: Clone + Debug + Eq + 'static,
        {
            let v2 = value.clone();
            Self::new(label, move |_, data| *data == value).with(move |cx, _| cx.push(v2.clone()))
        }

        fn direction(&self) -> Direction {
            match self.label.text().text_is_rtl() {
                false => Direction::Right,
                true => Direction::Left,
            }
        }
    }
}
