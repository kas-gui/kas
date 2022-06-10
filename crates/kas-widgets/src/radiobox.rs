// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::prelude::*;
use kas::theme::Feature;
use kas::updatable::{SharedRc, SingleData};
use log::trace;
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;

/// A group used by [`RadioBox`] and [`RadioBoxBare`]
///
/// This type can (and likely should) be default constructed.
pub type RadioGroup = SharedRc<Option<WidgetId>>;

impl_scope! {
    /// A bare radiobox (no label)
    #[autoimpl(Debug ignore self.on_select)]
    #[derive(Clone)]
    #[widget {
        key_nav = true;
        hover_highlight = true;
    }]
    pub struct RadioBoxBare {
        core: widget_core!(),
        state: bool,
        last_change: Option<Instant>,
        group: RadioGroup,
        on_select: Option<Rc<dyn Fn(&mut EventMgr)>>,
    }

    impl Widget for Self {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Update { id, .. } if id == self.group.id() => {
                    if self.state && !self.eq_id(self.group.get_cloned()) {
                        trace!("RadioBoxBare: unset {}", self.id());
                        self.state = false;
                        self.last_change = Some(Instant::now());
                        mgr.redraw(self.id());
                    }
                    Response::Used
                }
                event => event.on_activate(mgr, self.id(), |mgr| {
                    if !self.state {
                        trace!("RadioBoxBare: set {}", self.id());
                        self.state = true;
                        self.last_change = Some(Instant::now());
                        mgr.redraw(self.id());
                        self.group.update(mgr, Some(self.id()));
                        if let Some(f) = self.on_select.as_ref() {
                            f(mgr);
                        }
                    }
                    Response::Used
                })
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            size_mgr.feature(Feature::RadioBox, axis)
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            let rect = mgr.align_feature(Feature::RadioBox, rect, align);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.radiobox(self.rect(), self.state, self.last_change);
        }
    }

    impl Self {
        /// Construct a radiobox
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `group` will be considered part of a single group.
        #[inline]
        pub fn new(group: RadioGroup) -> Self {
            RadioBoxBare {
                core: Default::default(),
                state: false,
                last_change: None,
                group,
                on_select: None,
            }
        }

        /// Set event handler `f`
        ///
        /// When the radiobox is selected, the closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        #[must_use]
        pub fn on_select<F>(self, f: F) -> RadioBoxBare
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            RadioBoxBare {
                core: self.core,
                state: self.state,
                last_change: self.last_change,
                group: self.group,
                on_select: Some(Rc::new(f)),
            }
        }

        /// Construct a radiobox with given `group` and event handler `f`
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `group` will be considered part of a single group.
        ///
        /// When the radiobox is selected, the closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        pub fn new_on<F>(group: RadioGroup, f: F) -> Self
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            RadioBoxBare::new(group).on_select(f)
        }

        /// Set the initial state of the radiobox.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.state = state;
            self.last_change = None;
            self
        }

        /// Unset all radioboxes in the group
        ///
        /// Note: state will not update until the next draw.
        #[inline]
        pub fn unset_all(&self, mgr: &mut EventMgr) {
            self.group.update(mgr, None);
        }
    }

    impl HasBool for Self {
        fn get_bool(&self) -> bool {
            self.state
        }

        fn set_bool(&mut self, state: bool) -> TkAction {
            if state == self.state {
                return TkAction::empty();
            }

            self.state = state;
            self.last_change = None;
            TkAction::REDRAW
        }
    }
}

impl_scope! {
    /// A radiobox with label
    #[autoimpl(Debug)]
    #[autoimpl(HasBool using self.inner)]
    #[derive(Clone)]
    #[widget{
        layout = row: [self.inner, self.label];
    }]
    pub struct RadioBox {
        core: widget_core!(),
        #[widget]
        inner: RadioBoxBare,
        #[widget]
        label: AccelLabel,
    }

    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.inner.id())
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.inner.id_ref(), self.label.keys());
        }
    }

    impl Self {
        /// Construct a radiobox with a given `label` and `group`
        ///
        /// RadioBox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `group` will be considered part of a single group.
        #[inline]
        pub fn new<T: Into<AccelString>>(label: T, group: RadioGroup) -> Self {
            RadioBox {
                core: Default::default(),
                inner: RadioBoxBare::new(group),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// When the radiobox is selected, the closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        #[must_use]
        pub fn on_select<F>(self, f: F) -> RadioBox
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            RadioBox {
                core: self.core,
                inner: self.inner.on_select(f),
                label: self.label,
            }
        }

        /// Construct a radiobox with given `label`, `group` and event handler `f`
        ///
        /// RadioBox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `group` will be considered part of a single group.
        ///
        /// When the radiobox is selected, the closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, group: RadioGroup, f: F) -> Self
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            RadioBox::new(label, group).on_select(f)
        }

        /// Construct a radiobox with given `label`, `group` and payload `msg`
        ///
        /// RadioBox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `group` will be considered part of a single group.
        ///
        /// When the radiobox is selected, a clone
        /// of `msg` is returned to the parent widget via [`EventMgr::push_msg`].
        ///
        /// No handler is called on deselection.
        #[inline]
        pub fn new_msg<S, M: Clone>(label: S, group: RadioGroup, msg: M) -> Self
        where
            S: Into<AccelString>,
            M: Clone + Debug + 'static,
        {
            Self::new_on(label, group, move |mgr| mgr.push_msg(msg.clone()))
        }

        /// Set the initial state of the radiobox.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.inner = self.inner.with_state(state);
            self
        }

        /// Unset all radioboxes in the group
        ///
        /// Note: state will not update until the next draw.
        #[inline]
        pub fn unset_all(&self, mgr: &mut EventMgr) {
            self.inner.unset_all(mgr)
        }
    }
}
