// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::prelude::*;
use kas::theme::Feature;
use std::fmt::Debug;
use std::time::Instant;

impl_scope! {
    /// A bare check box (no label)
    ///
    /// See also [`CheckButton`] which includes a label.
    #[autoimpl(Debug ignore self.state_fn, self.on_toggle)]
    #[widget{
        navigable = true;
        hover_highlight = true;
    }]
    pub struct CheckBox<A> {
        core: widget_core!(),
        align: AlignPair,
        state: bool,
        editable: bool,
        last_change: Option<Instant>,
        state_fn: Box<dyn Fn(&ConfigCx, &A) -> bool>,
        on_toggle: Option<Box<dyn Fn(&mut EventCx, &A, bool)>>,
    }

    impl Events for Self {
        type Data = A;

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            let new_state = (self.state_fn)(cx, data);
            if self.state != new_state {
                self.state = new_state;
                self.last_change = Some(Instant::now());
                cx.redraw(self.id());
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &A, event: Event) -> Response {
            event.on_activate(cx, self.id(), |cx| {
                self.toggle(cx, data);
                Response::Used
            })
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            self.align.set_component(axis, axis.align_or_center());
            sizer.feature(Feature::CheckBox, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            let rect = cx.align_feature(Feature::CheckBox, rect, self.align);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.check_box(self.rect(), self.state, self.last_change);
        }
    }

    impl Self {
        /// Construct a check box
        ///
        /// - `state_fn` extracts the current state from input data
        #[inline]
        pub fn new(state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static) -> Self {
            CheckBox {
                core: Default::default(),
                align: Default::default(),
                state: false,
                editable: true,
                last_change: None,
                state_fn: Box::new(state_fn),
                on_toggle: None,
            }
        }

        /// Call the handler `f` on toggle
        #[inline]
        #[must_use]
        pub fn with(mut self, f: impl Fn(&mut EventCx, &A, bool) + 'static) -> Self {
            debug_assert!(self.on_toggle.is_none());
            self.on_toggle = Some(Box::new(f));
            self
        }

        /// Send the message generated by `f` on toggle
        #[inline]
        #[must_use]
        pub fn with_msg<M>(self, f: impl Fn(bool) -> M + 'static) -> Self
        where
            M: std::fmt::Debug + 'static,
        {
            self.with(move |cx, _, state| cx.push(f(state)))
        }

        /// Construct a check box
        ///
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `msg_fn` is emitted when toggled
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
            msg_fn: impl Fn(bool) -> M + 'static,
        ) -> Self {
            CheckBox::new(state_fn).with_msg(msg_fn)
        }

        /// Set whether this widget is editable (inline)
        #[inline]
        #[must_use]
        pub fn with_editable(mut self, editable: bool) -> Self {
            self.editable = editable;
            self
        }

        /// Get whether this widget is editable
        #[inline]
        pub fn is_editable(&self) -> bool {
            self.editable
        }

        /// Set whether this widget is editable
        #[inline]
        pub fn set_editable(&mut self, editable: bool) {
            self.editable = editable;
        }

        /// Toggle the check box
        pub fn toggle(&mut self, cx: &mut EventCx, data: &A) {
            // Note: do not update self.state; that is the responsibility of update.
            self.state = !self.state;
            if let Some(f) = self.on_toggle.as_ref() {
                // Pass what should be the new value of state here:
                f(cx, data, self.state);
            }

            // Do animate (even if state never changes):
            self.last_change = Some(Instant::now());
            cx.redraw(self.id());
        }
    }
}

// Shrink left/right edge to only make portion with text clickable.
// This is a little hacky since neither Label widgets nor row
// layouts shrink self due to unused space.
// We don't shrink vertically since normally that isn't an issue.
pub(crate) fn shrink_to_text(rect: &mut Rect, direction: Direction, label: &AccelLabel) {
    if let Ok(bb) = label.text().bounding_box() {
        match direction {
            Direction::Right => {
                let offset = label.rect().pos.0 - rect.pos.0;
                let text_right: i32 = ((bb.1).0).cast_ceil();
                rect.size.0 = offset + text_right;
            }
            Direction::Left => {
                let text_left: i32 = ((bb.0).0).cast_floor();
                rect.pos.0 += text_left;
                rect.size.0 -= text_left
            }
            _ => (),
        }
    }
}

impl_scope! {
    /// A check button with label
    ///
    /// This is a [`CheckBox`] with a label.
    #[widget{
        layout = list!(self.direction(), [self.inner, non_navigable!(self.label)]);
    }]
    pub struct CheckButton<A> {
        core: widget_core!(),
        #[widget]
        inner: CheckBox<A>,
        #[widget(&())]
        label: AccelLabel,
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            <Self as kas::layout::AutoLayout>::set_rect(self, cx, rect);
            let dir = self.direction();
            shrink_to_text(&mut self.core.rect, dir, &self.label);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.inner.id())
        }
    }

    impl Events for Self {
        type Data = A;

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
            if let Some(kas::message::Activate) = cx.try_pop() {
                self.inner.toggle(cx, data);
            }
        }
    }

    impl Self {
        /// Construct a check button with the given `label`
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        #[inline]
        pub fn new(
            label: impl Into<AccelString>,
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
        ) -> Self {
            CheckButton {
                core: Default::default(),
                inner: CheckBox::new(state_fn),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Call the handler `f` on toggle
        #[inline]
        #[must_use]
        pub fn with(self, f: impl Fn(&mut EventCx, &A, bool) + 'static) -> Self {
            CheckButton {
                core: self.core,
                inner: self.inner.with(f),
                label: self.label,
            }
        }

        /// Send the message generated by `f` on toggle
        #[inline]
        #[must_use]
        pub fn with_msg<M>(self, f: impl Fn(bool) -> M + 'static) -> Self
        where
            M: std::fmt::Debug + 'static,
        {
            self.with(move |cx, _, state| cx.push(f(state)))
        }

        /// Construct a check button with the given `label` and `msg_fn`
        ///
        /// - `label` is displayed to the left or right (according to text direction)
        /// - `state_fn` extracts the current state from input data
        /// - A message generated by `msg_fn` is emitted when toggled
        #[inline]
        pub fn new_msg<M: Debug + 'static>(
            label: impl Into<AccelString>,
            state_fn: impl Fn(&ConfigCx, &A) -> bool + 'static,
            msg_fn: impl Fn(bool) -> M + 'static,
        ) -> Self {
            CheckButton::new(label, state_fn).with_msg(msg_fn)
        }

        /// Set whether this widget is editable (inline)
        #[inline]
        #[must_use]
        pub fn editable(mut self, editable: bool) -> Self {
            self.inner = self.inner.with_editable(editable);
            self
        }

        /// Get whether this widget is editable
        #[inline]
        pub fn is_editable(&self) -> bool {
            self.inner.is_editable()
        }

        /// Set whether this widget is editable
        #[inline]
        pub fn set_editable(&mut self, editable: bool) {
            self.inner.set_editable(editable);
        }

        fn direction(&self) -> Direction {
            match self.label.text().text_is_rtl() {
                Ok(false) | Err(_) => Direction::Right,
                Ok(true) => Direction::Left,
            }
        }
    }
}
