// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use std::convert::TryFrom;
use std::fmt::{self, Debug};
use std::rc::Rc;

use super::Label;
use crate::class::HasBool;
use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Action, Manager, Response, UpdateHandle, VoidMsg};
use crate::geom::Rect;
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::{Align, AlignHints, CoreData, CowString, Layout, WidgetConfig, WidgetCore, WidgetId};

/// A bare radiobox (no label)
#[handler(event)]
#[derive(Clone, Widget)]
pub struct RadioBoxBare<M> {
    #[widget_core]
    core: CoreData,
    state: bool,
    handle: UpdateHandle,
    on_activate: Option<Rc<dyn Fn(WidgetId) -> M>>,
}

impl<M> Debug for RadioBoxBare<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RadioBoxBare {{ core: {:?}, state: {:?}, handle: {:?}, ... }}",
            self.core, self.state, self.handle,
        )
    }
}

impl<M> WidgetConfig for RadioBoxBare<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.update_on_handle(self.handle, self.id());
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<M> event::Handler for RadioBoxBare<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn action(&mut self, mgr: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => {
                if !self.state {
                    self.state = true;
                    mgr.redraw(self.id());
                    mgr.trigger_update(self.handle, self.id().into());
                    if let Some(ref f) = self.on_activate {
                        f(self.id()).into()
                    } else {
                        Response::None
                    }
                } else {
                    Response::None
                }
            }
            Action::HandleUpdate { payload, .. } => {
                let id = WidgetId::try_from(payload).unwrap();
                if id != self.id() {
                    self.state = false;
                    mgr.redraw(self.id());
                }
                Response::None
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}

impl<M> Layout for RadioBoxBare<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.radiobox();
        self.core.rect.size = size;
        let margins = size_handle.outer_margins();
        SizeRules::extract_fixed(axis.dir(), size, margins)
    }

    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        let rect = align
            .complete(Align::Centre, Align::Centre, self.rect().size)
            .apply(rect);
        self.core.rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        let highlights = mgr.highlight_state(self.id());
        draw_handle.radiobox(self.core.rect, self.state, highlights);
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
            on_activate: None,
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_activate<M, F>(self, f: F) -> RadioBoxBare<M>
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBoxBare {
            core: self.core,
            state: self.state,
            handle: self.handle,
            on_activate: Some(Rc::new(f)),
        }
    }
}

impl<M> RadioBoxBare<M> {
    /// Construct a radiobox which calls `f` when toggled
    ///
    /// This is a shortcut for `RadioBoxBare::new().on_activate(f)`.
    ///
    /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
    /// same `handle` will be considered part of a single group.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on<F>(f: F, handle: UpdateHandle) -> Self
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBoxBare {
            core: Default::default(),
            state: false,
            handle,
            on_activate: Some(Rc::new(f)),
        }
    }

    /// Set the initial state of the radiobox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.state = state;
        self
    }
}

impl<M> HasBool for RadioBoxBare<M> {
    fn get_bool(&self) -> bool {
        self.state
    }

    fn set_bool(&mut self, mgr: &mut Manager, state: bool) {
        self.state = state;
        mgr.redraw(self.id());
        if state {
            mgr.trigger_update(self.handle, self.id().into());
        }
    }
}

/// A radiobox with optional label
#[layout(horizontal, area=radiobox)]
#[widget_config]
#[handler(msg = M, generics = <> where M: From<VoidMsg>)]
#[derive(Clone, Widget)]
pub struct RadioBox<M> {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget]
    radiobox: RadioBoxBare<M>,
    #[widget]
    label: Label,
}

impl<M> Debug for RadioBox<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RadioBox {{ core: {:?}, layout_data: {:?}, radiobox: {:?}, label: {:?} }}",
            self.core, self.layout_data, self.radiobox, self.label,
        )
    }
}

impl<M> RadioBox<M> {
    /// Construct a radiobox with a given `label` which calls `f` when toggled.
    ///
    /// This is a shortcut for `RadioBox::new(label).on_activate(f)`.
    ///
    /// Checkbox labels are optional; if no label is desired, use an empty
    /// string.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on<T: Into<CowString>, F>(f: F, handle: UpdateHandle, label: T) -> Self
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBox {
            core: Default::default(),
            layout_data: Default::default(),
            radiobox: RadioBoxBare::new_on(f, handle),
            label: Label::new(label),
        }
    }
}

impl RadioBox<VoidMsg> {
    /// Construct a radiobox with a given `label`.
    ///
    /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
    /// same `handle` will be considered part of a single group.
    ///
    /// RadioBox labels are optional; if no label is desired, use an empty
    /// string.
    #[inline]
    pub fn new<T: Into<CowString>>(handle: UpdateHandle, label: T) -> Self {
        RadioBox {
            core: Default::default(),
            layout_data: Default::default(),
            radiobox: RadioBoxBare::new(handle),
            label: Label::new(label),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_activate<M, F>(self, f: F) -> RadioBox<M>
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBox {
            core: self.core,
            layout_data: self.layout_data,
            radiobox: self.radiobox.on_activate(f),
            label: self.label,
        }
    }
}

impl<M> RadioBox<M> {
    /// Set the initial state of the radiobox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.radiobox = self.radiobox.state(state);
        self
    }
}

impl<M> HasBool for RadioBox<M> {
    #[inline]
    fn get_bool(&self) -> bool {
        self.radiobox.get_bool()
    }

    #[inline]
    fn set_bool(&mut self, mgr: &mut Manager, state: bool) {
        self.radiobox.set_bool(mgr, state);
    }
}
