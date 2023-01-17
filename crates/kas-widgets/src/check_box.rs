// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::prelude::*;
use kas::theme::Feature;
use std::rc::Rc;
use std::time::Instant;

impl_scope! {
    /// A bare check box (no label)
    ///
    /// See also [`CheckButton`] which includes a label.
    #[autoimpl(Debug ignore self.on_toggle)]
    #[derive(Clone, Default)]
    #[widget{
        navigable = true;
        hover_highlight = true;
    }]
    pub struct CheckBox {
        core: widget_core!(),
        align: AlignPair,
        state: bool,
        editable: bool,
        last_change: Option<Instant>,
        on_toggle: Option<Rc<dyn Fn(&mut EventMgr, bool)>>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.align.set_component(axis, axis.align_or_center());
            size_mgr.feature(Feature::CheckBox, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            let rect = mgr.align_feature(Feature::CheckBox, rect, self.align);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.check_box(self.rect(), self.state, self.last_change);
        }
    }

    impl Self {
        /// Construct a check box
        #[inline]
        pub fn new() -> Self {
            CheckBox {
                core: Default::default(),
                align: Default::default(),
                state: false,
                editable: true,
                last_change: None,
                on_toggle: None,
            }
        }

        /// Set event handler `f`
        ///
        /// When the check box is set or unset, the closure `f` is called.
        #[inline]
        #[must_use]
        pub fn on_toggle<F>(self, f: F) -> CheckBox
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckBox {
                core: self.core,
                align: self.align,
                state: self.state,
                editable: self.editable,
                last_change: self.last_change,
                on_toggle: Some(Rc::new(f)),
            }
        }

        /// Construct a check box with event handler `f`
        ///
        /// When the check box is set or unset, the closure `f` is called.
        #[inline]
        pub fn new_on<F>(f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckBox::new().on_toggle(f)
        }

        /// Set the initial state of the check box.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.state = state;
            self.last_change = None;
            self
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
    }

    impl HasBool for Self {
        fn get_bool(&self) -> bool {
            self.state
        }

        fn set_bool(&mut self, state: bool) -> Action {
            if state == self.state {
                return Action::empty();
            }

            self.state = state;
            self.last_change = None;
            Action::REDRAW
        }
    }

    impl Widget for Self {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            event.on_activate(mgr, self.id(), |mgr| {
                self.state = !self.state;
                self.last_change = Some(Instant::now());
                if let Some(f) = self.on_toggle.as_ref() {
                    f(mgr, self.state);
                }
                Response::Used
            })
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
    /// See also [`CheckBox`] which excludes the label.
    #[autoimpl(Debug)]
    #[autoimpl(HasBool using self.inner)]
    #[derive(Clone, Default)]
    #[widget{
        layout = list(self.direction()): [self.inner, non_navigable: self.label];
    }]
    pub struct CheckButton {
        core: widget_core!(),
        #[widget]
        inner: CheckBox,
        #[widget]
        label: AccelLabel,
    }

    impl Layout for Self {
        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            <Self as kas::layout::AutoLayout>::set_rect(self, mgr, rect);
            let dir = self.direction();
            shrink_to_text(&mut self.core.rect, dir, &self.label);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.inner.id())
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut ConfigMgr) {
            mgr.add_accel_keys(self.inner.id_ref(), self.label.keys());
        }
    }

    impl Self {
        /// Construct a check button with a given `label`
        ///
        /// CheckButton labels are optional; if no label is desired, use an empty
        /// string or use [`CheckBox`] instead.
        #[inline]
        pub fn new<T: Into<AccelString>>(label: T) -> Self {
            CheckButton {
                core: Default::default(),
                inner: CheckBox::new(),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// When the check button is set or unset, the closure `f` is called.
        #[inline]
        #[must_use]
        pub fn on_toggle<F>(self, f: F) -> CheckButton
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckButton {
                core: self.core,
                inner: self.inner.on_toggle(f),
                label: self.label,
            }
        }

        /// Construct a check button with a given `label` and event handler `f`
        ///
        /// CheckButton labels are optional; if no label is desired, use an empty
        /// string or use [`CheckBox`] instead.
        ///
        /// When the check button is set or unset, the closure `f` is called.
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckButton::new(label).on_toggle(f)
        }

        /// Set the initial state of the check button.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.inner = self.inner.with_state(state);
            self
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
