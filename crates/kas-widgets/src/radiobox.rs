// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::prelude::*;
use kas::updatable::{SharedRc, SingleData};
use log::trace;
use std::rc::Rc;

/// Type of radiobox group
pub type RadioBoxGroup = SharedRc<Option<WidgetId>>;

impl_scope! {
    /// A bare radiobox (no label)
    #[autoimpl(Debug ignore self.on_select)]
    #[derive(Clone)]
    #[widget]
    pub struct RadioBoxBare {
        #[widget_core]
        core: CoreData,
        state: bool,
        group: RadioBoxGroup,
        on_select: Option<Rc<dyn Fn(&mut EventMgr)>>,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            for handle in self.group.update_handles().into_iter() {
                mgr.update_on_handle(handle, self.id());
            }
        }

        fn key_nav(&self) -> bool {
            true
        }
        fn hover_highlight(&self) -> bool {
            true
        }
    }

    impl Handler for Self {
        #[inline]
        fn activation_via_press(&self) -> bool {
            true
        }

        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Activate => {
                    if !self.state {
                        trace!("RadioBoxBare: set {}", self.id());
                        self.state = true;
                        mgr.redraw(self.id());
                        if let Some(handle) = self.group.update(Some(self.id())) {
                            mgr.trigger_update(handle, 0);
                        }
                        if let Some(msg) = self.on_select.as_ref().map(|f| f(mgr)) {
                            mgr.push_msg(msg);
                        }
                    }
                    Response::Used
                }
                Event::HandleUpdate { .. } => {
                    if self.state && !self.eq_id(self.group.get_cloned()) {
                        trace!("RadioBoxBare: unset {}", self.id());
                        self.state = false;
                        mgr.redraw(self.id());
                    }
                    Response::Used
                }
                _ => Response::Unused,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let size = size_mgr.radiobox();
            self.core.rect.size = size;
            let margins = size_mgr.outer_margins();
            SizeRules::extract_fixed(axis, size, margins)
        }

        fn set_rect(&mut self, _: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            let rect = align
                .complete(Align::Center, Align::Center)
                .aligned_rect(self.rect().size, rect);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.radiobox(&*self, self.state);
        }
    }

    impl Self {
        /// Construct a radiobox
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `group` will be considered part of a single group.
        #[inline]
        pub fn new(group: RadioBoxGroup) -> Self {
            RadioBoxBare {
                core: Default::default(),
                state: false,
                group,
                on_select: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
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
                group: self.group,
                on_select: Some(Rc::new(f)),
            }
        }

        /// Construct a radiobox with given `group` and event handler `f`
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `group` will be considered part of a single group.
        ///
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        pub fn new_on<F>(group: RadioBoxGroup, f: F) -> Self
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
            self
        }

        /// Unset all radioboxes in the group
        ///
        /// Note: state will not update until the next draw.
        #[inline]
        pub fn unset_all(&self, mgr: &mut EventMgr) {
            if let Some(handle) = self.group.update(None) {
                mgr.trigger_update(handle, 0);
            }
        }
    }

    impl HasBool for Self {
        fn get_bool(&self) -> bool {
            self.state
        }

        fn set_bool(&mut self, state: bool) -> TkAction {
            self.state = state;
            TkAction::REDRAW
        }
    }
}

impl_scope! {
    /// A radiobox with label
    #[autoimpl(Debug)]
    #[autoimpl(HasBool using self.radiobox)]
    #[derive(Clone)]
    #[widget{
        find_id = Some(self.radiobox.id());
        layout = row: *;
    }]
    pub struct RadioBox {
        #[widget_core]
        core: CoreData,
        #[widget]
        radiobox: RadioBoxBare,
        #[widget]
        label: AccelLabel,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.radiobox.id_ref(), self.label.keys());
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
        pub fn new<T: Into<AccelString>>(label: T, group: RadioBoxGroup) -> Self {
            RadioBox {
                core: Default::default(),
                radiobox: RadioBoxBare::new(group),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
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
                radiobox: self.radiobox.on_select(f),
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
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        ///
        /// No handler is called on deselection.
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, group: RadioBoxGroup, f: F) -> Self
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
        /// On selection (through user input events or [`Event::Activate`]) a clone
        /// of `msg` is returned to the parent widget via [`EventMgr::push_msg`].
        ///
        /// No handler is called on deselection.
        #[inline]
        pub fn new_msg<S, M: Clone>(label: S, group: RadioBoxGroup, msg: M) -> Self
        where
            S: Into<AccelString>,
            M: Clone + 'static,
        {
            Self::new_on(label, group, move |mgr| mgr.push_msg(msg.clone()))
        }

        /// Set the initial state of the radiobox.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.radiobox = self.radiobox.with_state(state);
            self
        }

        /// Unset all radioboxes in the group
        ///
        /// Note: state will not update until the next draw.
        #[inline]
        pub fn unset_all(&self, mgr: &mut EventMgr) {
            self.radiobox.unset_all(mgr)
        }
    }
}
