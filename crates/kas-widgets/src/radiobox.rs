// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use super::AccelLabel;
use kas::prelude::*;
use log::trace;
use std::rc::Rc;

widget! {
    /// A bare radiobox (no label)
    #[autoimpl(Debug skip on_select)]
    #[derive(Clone)]
    pub struct RadioBoxBare<M: 'static> {
        #[widget_core]
        core: CoreData,
        state: bool,
        handle: UpdateHandle,
        on_select: Option<Rc<dyn Fn(&mut Manager) -> Option<M>>>,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            mgr.update_on_handle(self.handle, self.id());
        }

        fn key_nav(&self) -> bool {
            true
        }
        fn hover_highlight(&self) -> bool {
            true
        }
    }

    impl Handler for Self {
        type Msg = M;

        #[inline]
        fn activation_via_press(&self) -> bool {
            true
        }

        fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
            match event {
                Event::Activate => {
                    if !self.state {
                        trace!("RadioBoxBare: set {}", self.id());
                        self.state = true;
                        mgr.redraw(self.id());
                        mgr.trigger_update(self.handle, self.id().into());
                        Response::update_or_msg(self.on_select.as_ref().and_then(|f| f(mgr)))
                    } else {
                        Response::None
                    }
                }
                Event::HandleUpdate { payload, .. } => {
                    let opt_id = WidgetId::opt_from_u64(payload);
                    if self.state && opt_id != Some(self.id()) {
                        trace!("RadioBoxBare: unset {}", self.id());
                        self.state = false;
                        mgr.redraw(self.id());
                        Response::Update
                    } else {
                        Response::None
                    }
                }
                _ => Response::Unhandled,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            let size = size_handle.radiobox();
            self.core.rect.size = size;
            let margins = size_handle.outer_margins();
            SizeRules::extract_fixed(axis, size, margins)
        }

        fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
            let rect = align
                .complete(Align::Center, Align::Center)
                .aligned_rect(self.rect().size, rect);
            self.core.rect = rect;
        }

        fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            draw.radiobox(self.core.rect, self.state, self.input_state(mgr, disabled));
        }
    }

    impl RadioBoxBare<VoidMsg> {
        /// Construct a radiobox
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `handle` will be considered part of a single group.
        #[inline]
        pub fn new(handle: UpdateHandle) -> Self {
            RadioBoxBare {
                core: Default::default(),
                state: false,
                handle,
                on_select: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        ///
        /// No handler is called on deselection, but [`Response::Update`] is returned.
        #[inline]
        pub fn on_select<M, F>(self, f: F) -> RadioBoxBare<M>
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            RadioBoxBare {
                core: self.core,
                state: self.state,
                handle: self.handle,
                on_select: Some(Rc::new(f)),
            }
        }
    }

    impl Self {
        /// Construct a radiobox with given `handle` and event handler `f`
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `handle` will be considered part of a single group.
        ///
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        ///
        /// No handler is called on deselection, but [`Response::Update`] is returned.
        #[inline]
        pub fn new_on<F>(handle: UpdateHandle, f: F) -> Self
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            RadioBoxBare::new(handle).on_select(f)
        }

        /// Set the initial state of the radiobox.
        #[inline]
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
}

widget! {
    /// A radiobox with label
    #[autoimpl(Debug)]
    #[autoimpl(HasBool on radiobox)]
    #[derive(Clone)]
    #[widget{
        find_id = Some(self.radiobox.id());
        layout = row: *;
    }]
    pub struct RadioBox<M: 'static> {
        #[widget_core]
        core: CoreData,
        #[widget]
        radiobox: RadioBoxBare<M>,
        #[widget]
        label: AccelLabel,
    }

    impl Handler for Self where M: From<VoidMsg> {
        type Msg = M;
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            mgr.add_accel_keys(self.radiobox.id(), self.label.keys());
        }
    }

    impl RadioBox<VoidMsg> {
        /// Construct a radiobox with a given `label` and `handle`
        ///
        /// RadioBox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `handle` will be considered part of a single group.
        #[inline]
        pub fn new<T: Into<AccelString>>(label: T, handle: UpdateHandle) -> Self {
            RadioBox {
                core: Default::default(),
                radiobox: RadioBoxBare::new(handle),
                label: AccelLabel::new(label.into()),
            }
        }

        /// Set event handler `f`
        ///
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        ///
        /// No handler is called on deselection, but [`Response::Update`] is returned.
        #[inline]
        pub fn on_select<M, F>(self, f: F) -> RadioBox<M>
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            RadioBox {
                core: self.core,
                radiobox: self.radiobox.on_select(f),
                label: self.label,
            }
        }
    }

    impl Self {
        /// Construct a radiobox with given `label`, `handle` and event handler `f`
        ///
        /// RadioBox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `handle` will be considered part of a single group.
        ///
        /// On selection (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
        ///
        /// No handler is called on deselection, but [`Response::Update`] is returned.
        #[inline]
        pub fn new_on<T: Into<AccelString>, F>(label: T, handle: UpdateHandle, f: F) -> Self
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            RadioBox::new(label, handle).on_select(f)
        }

        /// Construct a radiobox with given `label`, `handle` and payload `msg`
        ///
        /// RadioBox labels are optional; if no label is desired, use an empty
        /// string.
        ///
        /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
        /// same `handle` will be considered part of a single group.
        ///
        /// On selection (through user input events or [`Event::Activate`]) a clone
        /// of `msg` is returned to the parent widget via [`Response::Msg`].
        ///
        /// No handler is called on deselection, but [`Response::Update`] is returned.
        #[inline]
        pub fn new_msg<S: Into<AccelString>>(label: S, handle: UpdateHandle, msg: M) -> Self
        where
            M: Clone,
        {
            Self::new_on(label, handle, move |_| Some(msg.clone()))
        }

        /// Set the initial state of the radiobox.
        #[inline]
        pub fn with_state(mut self, state: bool) -> Self {
            self.radiobox = self.radiobox.with_state(state);
            self
        }
    }
}
