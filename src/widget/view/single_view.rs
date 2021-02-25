// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{DefaultView, SingleData, SingleDataMut, ViewWidget};
use kas::prelude::*;
use std::fmt::{self};

/// Single view widget
#[derive(Clone, Widget)]
#[widget(config=noauto)]
#[layout(single)]
#[handler(handle=noauto)]
pub struct SingleView<D: SingleData + 'static, W = <<D as SingleData>::Item as DefaultView>::Widget>
where
    W: ViewWidget<D::Item>,
{
    #[widget_core]
    core: CoreData,
    data: D,
    #[widget]
    child: W,
}

impl<D: SingleData + 'static + Default, W: ViewWidget<D::Item>> Default for SingleView<D, W> {
    fn default() -> Self {
        let data = D::default();
        let child = W::new(data.get_cloned());
        SingleView {
            core: Default::default(),
            data,
            child,
        }
    }
}

impl<D: SingleData + 'static, W: ViewWidget<D::Item>> SingleView<D, W> {
    /// Construct a new instance
    pub fn new(data: D) -> Self {
        let child = W::new(data.get_cloned());
        SingleView {
            core: Default::default(),
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

impl<D: SingleDataMut + 'static, W: ViewWidget<D::Item>> SingleView<D, W> {
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

impl<D: SingleData + 'static, W: ViewWidget<D::Item>> WidgetConfig for SingleView<D, W> {
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
    }
}

impl<D: SingleData + 'static, W: ViewWidget<D::Item>> Handler for SingleView<D, W> {
    type Msg = <W as Handler>::Msg;
    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::HandleUpdate { .. } => {
                let value = self.data.get_cloned();
                *mgr |= self.child.set(value);
                Response::None
            }
            event => Response::Unhandled(event),
        }
    }
}

impl<D: SingleData + 'static, W: ViewWidget<D::Item>> fmt::Debug for SingleView<D, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, data: {:?}, child: {:?} }}",
            self.core, self.data, self.child,
        )
    }
}
