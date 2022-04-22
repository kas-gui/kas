// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::{event, prelude::*};
use std::rc::Rc;

impl_scope! {
    /// A bare checkbox (no label)
    #[autoimpl(Debug ignore self.on_toggle)]
    #[derive(Clone, Default)]
    #[widget{
        key_nav = true;
        hover_highlight = true;
    }]
    pub struct CheckBoxBare {
        #[widget_core]
        core: CoreData,
        state: bool,
        editable: bool,
        on_toggle: Option<Rc<dyn Fn(&mut EventMgr, bool)>>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let size = size_mgr.checkbox();
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
            draw.checkbox(&*self, self.state);
        }
    }

    impl Self {
        /// Construct a checkbox
        #[inline]
        pub fn new() -> Self {
            CheckBoxBare {
                core: Default::default(),
                state: false,
                editable: true,
                on_toggle: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        #[must_use]
        pub fn on_toggle<F>(self, f: F) -> CheckBoxBare
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckBoxBare {
                core: self.core,
                state: self.state,
                editable: self.editable,
                on_toggle: Some(Rc::new(f)),
            }
        }

        /// Construct a checkbox with event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        pub fn new_on<F>(f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckBoxBare::new().on_toggle(f)
        }

        /// Set the initial state of the checkbox.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.state = state;
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

        fn set_bool(&mut self, state: bool) -> TkAction {
            self.state = state;
            TkAction::REDRAW
        }
    }

    impl event::Handler for Self {
        #[inline]
        fn activation_via_press(&self) -> bool {
            true
        }

        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Activate if self.editable => {
                    self.state = !self.state;
                    mgr.redraw(self.id());
                    self.on_toggle.as_ref().map(|f| f(mgr, self.state));
                    Response::Used
                }
                _ => Response::Unused,
            }
        }
    }
}

impl_scope! {
    /// A checkbox with label
    #[autoimpl(Debug)]
    #[autoimpl(HasBool using self.inner)]
    #[derive(Clone, Default)]
    #[widget{
        layout = row: *;
        find_id = Some(self.inner.id());
    }]
    pub struct CheckBox {
        #[widget_core]
        core: CoreData,
        #[widget]
        inner: CheckBoxBare,
        #[widget]
        label: AccelLabel,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.inner.id_ref(), self.label.keys());
        }
    }

    impl Self {
        /// Construct a checkbox with a given `label`
        ///
        /// CheckBox labels are optional; if no label is desired, use an empty
        /// string.
        #[inline]
        pub fn new<T: Into<AccelString>>(label: T) -> Self {
            CheckBox {
                core: Default::default(),
                inner: CheckBoxBare::new(),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        #[must_use]
        pub fn on_toggle<F>(self, f: F) -> CheckBox
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckBox {
                core: self.core,
                inner: self.inner.on_toggle(f),
                label: self.label,
            }
        }

        /// Construct a checkbox with a given `label` and event handler `f`
        ///
        /// CheckBox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) + 'static,
        {
            CheckBox::new(label).on_toggle(f)
        }

        /// Set the initial state of the checkbox.
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
    }
}
