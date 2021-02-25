// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{DefaultView, SingleData, SingleDataMut, View};
use kas::prelude::*;
use std::fmt::{self};

/// Single view widget
#[derive(Clone, Widget)]
#[widget(config=noauto)]
#[layout(single)]
#[handler(handle=noauto)]
pub struct SingleView<D: SingleData + 'static, V: View<D::Item> = DefaultView> {
    #[widget_core]
    core: CoreData,
    view: V,
    data: D,
    #[widget]
    child: V::Widget,
}

impl<D: SingleData + 'static + Default, V: View<D::Item> + Default> Default for SingleView<D, V> {
    fn default() -> Self {
        let view = <V as Default>::default();
        let data = D::default();
        let child = view.new(data.get_cloned());
        SingleView {
            core: Default::default(),
            view,
            data,
            child,
        }
    }
}
impl<D: SingleData + 'static, V: View<D::Item> + Default> SingleView<D, V> {
    /// Construct a new instance
    pub fn new(data: D) -> Self {
        Self::new_with_view(<V as Default>::default(), data)
    }
}
impl<D: SingleData + 'static, V: View<D::Item>> SingleView<D, V> {
    /// Construct a new instance with explicit view
    pub fn new_with_view(view: V, data: D) -> Self {
        let child = view.new(data.get_cloned());
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
}

impl<D: SingleDataMut + 'static, V: View<D::Item>> SingleView<D, V> {
    /// Set shared data
    ///
    /// Other widgets sharing this data are notified of the update.
    pub fn set_value(&self, mgr: &mut Manager, data: D::Item) {
        let handle = self.data.set(data);
        mgr.trigger_update(handle, 0);
    }

    /// Update shared data
    ///
    /// This is purely a convenience method over [`SingleView::get_value`] and
    /// [`SingleView::set_value`]. It always notifies other widgets sharing the data.
    pub fn update_value<F: Fn(D::Item) -> D::Item>(&self, mgr: &mut Manager, f: F) {
        self.set_value(mgr, f(self.get_value()));
    }
}

impl<D: SingleData + 'static, V: View<D::Item>> WidgetConfig for SingleView<D, V> {
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
    }
}

impl<D: SingleData + 'static, V: View<D::Item>> Handler for SingleView<D, V> {
    type Msg = <V::Widget as Handler>::Msg;
    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::HandleUpdate { .. } => {
                let value = self.data.get_cloned();
                *mgr |= self.view.set(&mut self.child, value);
                Response::None
            }
            event => Response::Unhandled(event),
        }
    }
}

impl<D: SingleData + 'static, V: View<D::Item>> fmt::Debug for SingleView<D, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, data: {:?}, child: {:?} }}",
            self.core, self.data, self.child,
        )
    }
}
