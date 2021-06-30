// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use log::trace;
use std::convert::TryFrom;
use std::fmt::{self, Debug};
use std::rc::Rc;

use super::AccelLabel;
use kas::prelude::*;

/// A bare radiobox (no label)
#[derive(Clone, Widget)]
#[handler(handle=noauto)]
#[widget(config=noauto)]
pub struct RadioBoxBare<M: 'static> {
    #[widget_core]
    core: CoreData,
    state: bool,
    handle: UpdateHandle,
    on_select: Option<Rc<dyn Fn(&mut Manager) -> Option<M>>>,
}

impl<M: 'static> Debug for RadioBoxBare<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RadioBox")
            .field("core", &self.core)
            .field("state", &self.state)
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

impl<M: 'static> WidgetConfig for RadioBoxBare<M> {
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

impl<M: 'static> Handler for RadioBoxBare<M> {
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
                let id = WidgetId::try_from(payload).unwrap();
                if self.state && id != self.id() {
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

impl<M: 'static> Layout for RadioBoxBare<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.radiobox();
        self.core.rect.size = size;
        let margins = size_handle.outer_margins();
        SizeRules::extract_fixed(axis, size, margins)
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        let rect = align
            .complete(Align::Centre, Align::Centre)
            .aligned_rect(self.rect().size, rect);
        self.core.rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        draw_handle.radiobox(self.core.rect, self.state, self.input_state(mgr, disabled));
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

impl<M: 'static> RadioBoxBare<M> {
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

impl<M: 'static> HasBool for RadioBoxBare<M> {
    fn get_bool(&self) -> bool {
        self.state
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.state = state;
        TkAction::REDRAW
    }
}

/// A radiobox with optional label
#[derive(Clone, Widget)]
#[layout(row, area=radiobox)]
#[handler(msg = M, generics = <> where M: From<VoidMsg>)]
#[widget(config=noauto)]
#[widget_derive(HasBool)]
pub struct RadioBox<M: 'static> {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget_derive]
    #[widget]
    radiobox: RadioBoxBare<M>,
    #[widget]
    label: AccelLabel,
}

impl<M: 'static> Debug for RadioBox<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RadioBox")
            .field("core", &self.core)
            .field("layout_data", &self.layout_data)
            .field("radiobox", &self.radiobox)
            .field("label", &self.label)
            .finish()
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
            layout_data: Default::default(),
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
            layout_data: self.layout_data,
            radiobox: self.radiobox.on_select(f),
            label: self.label,
        }
    }
}

impl<M: 'static> RadioBox<M> {
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
}

impl<M: 'static> RadioBox<M> {
    /// Set the initial state of the radiobox.
    #[inline]
    pub fn with_state(mut self, state: bool) -> Self {
        self.radiobox = self.radiobox.with_state(state);
        self
    }
}

impl<M: 'static> WidgetConfig for RadioBox<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.radiobox.id(), self.label.keys());
    }
}
