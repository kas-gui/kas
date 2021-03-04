// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{DefaultView, SingleData, View};
use kas::prelude::*;
use std::fmt::{self};

/// Single view widget
#[derive(Clone, Widget)]
#[widget(config=noauto)]
#[layout(single)]
#[handler(handle=noauto, send=noauto)]
pub struct SingleView<D: SingleData + 'static, V: View<(), D::Item> = DefaultView> {
    #[widget_core]
    core: CoreData,
    view: V,
    data: D,
    #[widget]
    child: V::Widget,
}

impl<D: SingleData + 'static + Default, V: View<(), D::Item> + Default> Default
    for SingleView<D, V>
{
    fn default() -> Self {
        let view = <V as Default>::default();
        let data = D::default();
        let child = view.new((), data.get_cloned());
        SingleView {
            core: Default::default(),
            view,
            data,
            child,
        }
    }
}
impl<D: SingleData + 'static, V: View<(), D::Item> + Default> SingleView<D, V> {
    /// Construct a new instance
    pub fn new(data: D) -> Self {
        Self::new_with_view(<V as Default>::default(), data)
    }
}
impl<D: SingleData + 'static, V: View<(), D::Item>> SingleView<D, V> {
    /// Construct a new instance with explicit view
    pub fn new_with_view(view: V, data: D) -> Self {
        let child = view.new((), data.get_cloned());
        SingleView {
            core: Default::default(),
            view,
            data,
            child,
        }
    }

    /// Access the data object
    pub fn data(&self) -> &D {
        &self.data
    }

    /// Access the data object (mut)
    pub fn data_mut(&mut self) -> &mut D {
        &mut self.data
    }

    /// Get a copy of the shared value
    pub fn get_value(&self) -> D::Item {
        self.data.get_cloned()
    }

    /// Set shared data
    ///
    /// This method updates the shared data, if supported (see
    /// [`SingleData::update`]). Other widgets sharing this data are notified
    /// of the update, if data is changed.
    pub fn set_value(&self, mgr: &mut Manager, data: D::Item) {
        if let Some(handle) = self.data.update(data) {
            mgr.trigger_update(handle, 0);
        }
    }

    /// Update shared data
    ///
    /// This is purely a convenience method over [`SingleView::set_value`].
    /// It notifies other widgets of updates to the shared data.
    pub fn update_value<F: Fn(D::Item) -> D::Item>(&self, mgr: &mut Manager, f: F) {
        self.set_value(mgr, f(self.get_value()));
    }
}

impl<D: SingleData + 'static, V: View<(), D::Item>> WidgetConfig for SingleView<D, V> {
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
    }
}

impl<D: SingleData + 'static, V: View<(), D::Item>> Handler for SingleView<D, V> {
    type Msg = <V::Widget as Handler>::Msg;
    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::HandleUpdate { .. } => {
                let value = self.data.get_cloned();
                *mgr |= self.view.set(&mut self.child, (), value);
                Response::Update
            }
            _ => Response::Unhandled,
        }
    }
}

impl<D: SingleData + 'static, V: View<(), D::Item>> SendEvent for SingleView<D, V> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled;
        }

        if id < self.id() {
            let r = self.child.send(mgr, id, event);
            match r {
                Response::Update | Response::Msg(_) => {
                    if let Some(item) = self.view.get(&self.child, &()) {
                        self.set_value(mgr, item);
                    }
                }
                _ => (),
            }
            r
        } else {
            debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
            self.handle(mgr, event)
        }
    }
}

impl<D: SingleData + 'static, V: View<(), D::Item>> fmt::Debug for SingleView<D, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, data: {:?}, child: {:?} }}",
            self.core, self.data, self.child,
        )
    }
}
