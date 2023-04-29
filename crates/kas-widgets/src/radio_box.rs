// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::prelude::*;
use kas::theme::Feature;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

impl_scope! {
    /// A bare radio box (no label)
    ///
    /// See also [`RadioButton`] which includes a label.
    #[autoimpl(Debug ignore self.state_fn, self.on_select)]
    #[widget {
        data = A;
        navigable = true;
        hover_highlight = true;
    }]
    pub struct RadioBox<A> {
        core: widget_core!(),
        align: AlignPair,
        state: bool,
        last_change: Option<Instant>,
        state_fn: Box<dyn Fn(&A) -> bool>,
        on_select: Option<Box<dyn Fn(&mut EventCx<A>)>>,
        _data: PhantomData<A>,
    }

    impl Widget for Self {
        fn update(&mut self, cx: &mut ConfigCx<A>) {
            let new_state = (self.state_fn)(cx.data());
            if self.state != new_state {
                self.state = new_state;
                self.last_change = Some(Instant::now());
                cx.redraw(self.id());
            }
        }

        fn handle_event(&mut self, mgr: &mut EventCx<A>, event: Event) -> Response {
            match event {
                event => event.on_activate(mgr, self.id(), |mgr| {
                    self.select(mgr);
                    Response::Used
                }),
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.align.set_component(axis, axis.align_or_center());
            size_mgr.feature(Feature::RadioBox, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            let rect = mgr.align_feature(Feature::RadioBox, rect, self.align);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.radio_box(self.rect(), self.state, self.last_change);
        }
    }

    impl Self {
        /// Construct a radio box
        ///
        /// - `state_fn` extracts the current state from input data
        #[inline]
        pub fn new(state_fn: impl Fn(&A) -> bool + 'static) -> Self {
            RadioBox {
                core: Default::default(),
                align: Default::default(),
                state: false,
                last_change: None,
                state_fn: Box::new(state_fn),
                on_select: None,
                _data: PhantomData,
            }
        }

        /// Set event handler `f`
        ///
        /// When the radio box is selected, the closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        #[must_use]
        pub fn on_select(self, f: impl Fn(&mut EventCx<A>) + 'static) -> Self {
            RadioBox {
                core: self.core,
                align: self.align,
                state: self.state,
                last_change: self.last_change,
                state_fn: self.state_fn,
                on_select: Some(Box::new(f)),
                _data: PhantomData,
            }
        }

        /// Construct a radio box
        ///
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `message_fn` is emitted when selected
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            state_fn: impl Fn(&A) -> bool + 'static,
            message_fn: impl Fn() -> M + 'static,
        ) -> Self {
            RadioBox::new(state_fn)
                .on_select(move |cx| cx.push(message_fn()))
        }

        fn select(&mut self, cx: &mut EventCx<A>) {
            self.last_change = Some(Instant::now());
            cx.redraw(self.id());
            if let Some(ref f) = self.on_select {
                f(cx);
            }
        }
    }
}

impl_scope! {
    /// A radio button with label
    ///
    /// See also [`RadioBox`] which excludes the label.
    #[autoimpl(Debug)]
    #[widget{
        data = A;
        layout = list(self.direction()): [self.inner, non_navigable: self.label];
    }]
    pub struct RadioButton<A> {
        core: widget_core!(),
        #[widget]
        inner: RadioBox<A>,
        #[widget(&())]
        label: AccelLabel,
    }

    impl Layout for Self {
        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            <Self as kas::layout::AutoLayout>::set_rect(self, mgr, rect);
            let dir = self.direction();
            crate::check_box::shrink_to_text(&mut self.core.rect, dir, &self.label);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.inner.id())
        }
    }

    impl Widget for Self {
        fn handle_message(&mut self, cx: &mut EventCx<A>) {
            if let Some(kas::message::Activate) = cx.try_pop() {
                self.inner.select(cx);
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
            label: impl Into<AccelString>,
            state_fn: impl Fn(&A) -> bool + 'static,
        ) -> Self {
            RadioButton {
                core: Default::default(),
                inner: RadioBox::new(state_fn),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// When the radio button is selected, the closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        #[must_use]
        pub fn on_select<F>(self, f: F) -> Self
        where
            F: Fn(&mut EventCx<A>) + 'static,
        {
            RadioButton {
                core: self.core,
                inner: self.inner.on_select(f),
                label: self.label,
            }
        }

        /// Construct a radio box
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `message_fn` is emitted when selected
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            label: impl Into<AccelString>,
            state_fn: impl Fn(&A) -> bool + 'static,
            message_fn: impl Fn() -> M + 'static,
        ) -> Self {
            RadioButton::new(label, state_fn)
                .on_select(move |cx| cx.push(message_fn()))
        }

        fn direction(&self) -> Direction {
            match self.label.text().text_is_rtl() {
                Ok(false) | Err(_) => Direction::Right,
                Ok(true) => Direction::Left,
            }
        }
    }
}
