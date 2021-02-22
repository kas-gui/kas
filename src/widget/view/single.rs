// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{DefaultView, ViewWidget};
use kas::event::UpdateHandle;
use kas::prelude::*;
use std::fmt::{self, Debug};

/// Trait for viewable single data items
// Note: we require Debug + 'static to allow widgets using this to implement
// WidgetCore, which requires Debug + Any.
pub trait SingleData: Debug + 'static {
    type Item: Clone;

    // TODO(gat): add get<'a>(&self) -> Self::ItemRef<'a> and get_mut

    /// Get data (clone)
    fn get_cloned(&self) -> Self::Item;

    /// Get an update handle, if any is used
    ///
    /// Widgets may use this `handle` to call `mgr.update_on_handle(handle, self.id())`.
    fn update_handle(&self) -> Option<UpdateHandle> {
        None
    }
}

/// Trait for writable single data items
pub trait SingleDataMut: SingleData {
    /// Set data
    fn set(&self, value: Self::Item) -> UpdateHandle;
}

/// Single view widget
#[derive(Clone, Widget)]
#[widget(config=noauto)]
#[layout(single)]
#[handler(handle=noauto)]
pub struct SingleView<D: SingleData, W = <<D as SingleData>::Item as DefaultView>::Widget>
where
    W: ViewWidget<D::Item>,
{
    #[widget_core]
    core: CoreData,
    data: D,
    #[widget]
    child: W,
}

impl<D: SingleData + Default, W: ViewWidget<D::Item>> Default for SingleView<D, W> {
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

impl<D: SingleData, W: ViewWidget<D::Item>> SingleView<D, W> {
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

impl<D: SingleDataMut, W: ViewWidget<D::Item>> SingleView<D, W> {
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

impl<D: SingleData, W: ViewWidget<D::Item>> WidgetConfig for SingleView<D, W> {
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.data.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
    }
}

impl<D: SingleData, W: ViewWidget<D::Item>> Handler for SingleView<D, W> {
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

impl<D: SingleData, W: ViewWidget<D::Item>> fmt::Debug for SingleView<D, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, data: {:?}, child: {:?} }}",
            self.core, self.data, self.child,
        )
    }
}
