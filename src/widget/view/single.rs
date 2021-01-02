// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{Accessor, AccessorShared, DefaultView, ViewWidget};
use kas::prelude::*;
use std::fmt;

/// Single view widget
#[derive(Clone, Widget)]
#[widget(config=noauto)]
#[layout(single)]
#[handler(handle=noauto)]
pub struct SingleView<A: Accessor<()>, W = <<A as Accessor<()>>::Item as DefaultView>::Widget>
where
    W: ViewWidget<<A as Accessor<()>>::Item>,
{
    #[widget_core]
    core: CoreData,
    accessor: A,
    #[widget]
    child: W,
}

impl<A: Accessor<()> + Default, W: ViewWidget<A::Item>> Default for SingleView<A, W> {
    fn default() -> Self {
        let accessor = A::default();
        let child = W::new(accessor.get(()));
        SingleView {
            core: Default::default(),
            accessor,
            child,
        }
    }
}

impl<A: Accessor<()>, W: ViewWidget<A::Item>> SingleView<A, W> {
    /// Construct a new instance
    pub fn new(accessor: A) -> Self {
        let child = W::new(accessor.get(()));
        SingleView {
            core: Default::default(),
            accessor,
            child,
        }
    }

    /// Get the data accessor
    pub fn accessor(&self) -> &A {
        &self.accessor
    }

    /// Get a copy of the shared value
    pub fn get_value(&self) -> A::Item {
        self.accessor.get(())
    }
}

impl<A: AccessorShared<()>, W: ViewWidget<A::Item>> SingleView<A, W> {
    /// Set shared data
    ///
    /// Other widgets sharing this data are notified of the update.
    pub fn set_value(&self, mgr: &mut Manager, data: A::Item) {
        let handle = self.accessor.set((), data);
        mgr.trigger_update(handle, 0);
    }

    /// Update shared data
    ///
    /// This is purely a convenience method over [`SingleView::get_value`] and
    /// [`SingleView::set_value`]. It always notifies other widgets sharing the data.
    pub fn update_value<F: Fn(A::Item) -> A::Item>(&self, mgr: &mut Manager, f: F) {
        self.set_value(mgr, f(self.get_value()));
    }
}

impl<A: Accessor<()>, W: ViewWidget<A::Item>> WidgetConfig for SingleView<A, W> {
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.accessor.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
    }
}

impl<A: Accessor<()>, W: ViewWidget<A::Item>> Handler for SingleView<A, W> {
    type Msg = <W as Handler>::Msg;
    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::HandleUpdate { .. } => {
                let value = self.accessor.get(());
                *mgr += self.child.set(value);
                Response::None
            }
            event => Response::Unhandled(event),
        }
    }
}

impl<A: Accessor<()>, W: ViewWidget<A::Item>> fmt::Debug for SingleView<A, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, accessor: {:?}, child: {:?} }}",
            self.core, self.accessor, self.child,
        )
    }
}
