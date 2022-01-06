// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::{event, prelude::*};
use std::rc::Rc;

widget! {
    /// A bare checkbox (no label)
    #[autoimpl(Debug skip on_toggle)]
    #[derive(Clone, Default)]
    #[widget{
        key_nav = true;
        hover_highlight = true;
    }]
    pub struct CheckBoxBare<M: 'static> {
        #[widget_core]
        core: CoreData,
        state: bool,
        on_toggle: Option<Rc<dyn Fn(&mut EventMgr, bool) -> Option<M>>>,
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let size = size_mgr.checkbox();
            self.core.rect.size = size;
            let margins = size_mgr.outer_margins();
            SizeRules::extract_fixed(axis, size, margins)
        }

        fn set_rect(&mut self, _: &mut EventMgr, rect: Rect, align: AlignHints) {
            let rect = align
                .complete(Align::Center, Align::Center)
                .aligned_rect(self.rect().size, rect);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr, disabled: bool) {
            draw.checkbox(self.core.rect, self.state, draw.input_state(self, disabled));
        }
    }

    impl CheckBoxBare<VoidMsg> {
        /// Construct a checkbox
        #[inline]
        pub fn new() -> Self {
            CheckBoxBare {
                core: Default::default(),
                state: false,
                on_toggle: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        #[inline]
        #[must_use]
        pub fn on_toggle<M, F>(self, f: F) -> CheckBoxBare<M>
        where
            F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
        {
            CheckBoxBare {
                core: self.core,
                state: self.state,
                on_toggle: Some(Rc::new(f)),
            }
        }
    }

    impl Self {
        /// Construct a checkbox with event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        #[inline]
        pub fn new_on<F>(f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
        {
            CheckBoxBare::new().on_toggle(f)
        }
    }

    impl Self {
        /// Set the initial state of the checkbox.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.state = state;
            self
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
        type Msg = M;

        #[inline]
        fn activation_via_press(&self) -> bool {
            true
        }

        fn handle(&mut self, mgr: &mut EventMgr, event: Event) -> Response<M> {
            match event {
                Event::Activate => {
                    self.state = !self.state;
                    mgr.redraw(self.id());
                    Response::update_or_msg(self.on_toggle.as_ref().and_then(|f| f(mgr, self.state)))
                }
                _ => Response::Unused,
            }
        }
    }
}

widget! {
    /// A checkbox with label
    #[autoimpl(Debug)]
    #[autoimpl(HasBool on checkbox)]
    #[derive(Clone, Default)]
    #[widget{
        layout = row: *;
        find_id = Some(self.checkbox.id());
    }]
    pub struct CheckBox<M: 'static> {
        #[widget_core]
        core: CoreData,
        #[widget]
        checkbox: CheckBoxBare<M>,
        #[widget]
        label: AccelLabel,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut EventMgr) {
            mgr.add_accel_keys(self.checkbox.id(), self.label.keys());
        }
    }

    impl Handler for Self where M: From<VoidMsg> {
        type Msg = M;
    }

    impl CheckBox<VoidMsg> {
        /// Construct a checkbox with a given `label`
        ///
        /// CheckBox labels are optional; if no label is desired, use an empty
        /// string.
        #[inline]
        pub fn new<T: Into<AccelString>>(label: T) -> Self {
            CheckBox {
                core: Default::default(),
                checkbox: CheckBoxBare::new(),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        #[inline]
        #[must_use]
        pub fn on_toggle<M, F>(self, f: F) -> CheckBox<M>
        where
            F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
        {
            CheckBox {
                core: self.core,
                checkbox: self.checkbox.on_toggle(f),
                label: self.label,
            }
        }
    }

    impl Self {
        /// Construct a checkbox with a given `label` and event handler `f`
        ///
        /// Checkbox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// On toggle (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, f: F) -> Self
        where
            F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
        {
            CheckBox::new(label).on_toggle(f)
        }

        /// Set the initial state of the checkbox.
        #[inline]
        #[must_use]
        pub fn with_state(mut self, state: bool) -> Self {
            self.checkbox = self.checkbox.with_state(state);
            self
        }
    }
}
