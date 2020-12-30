// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{Accessor, AccessorShared, DefaultView, ViewWidget};
use kas::prelude::*;
use std::fmt;
use std::marker::PhantomData;

/// Single view widget
#[derive(Clone, Default, Widget)]
#[widget(config=noauto)]
#[layout(single)]
#[handler(msg=<W as Handler>::Msg)]
pub struct SingleView<T: 'static, A: Accessor<(), T>, W: ViewWidget<T> = <T as DefaultView>::Widget>
{
    #[widget_core]
    core: CoreData,
    _t: PhantomData<T>,
    accessor: A,
    #[widget]
    child: W,
}

impl<T: 'static, A: Accessor<(), T>, W: ViewWidget<T>> SingleView<T, A, W> {
    /// Construct a new instance
    pub fn new(accessor: A) -> Self {
        let child = W::new(accessor.get(()));
        SingleView {
            core: Default::default(),
            _t: Default::default(),
            accessor,
            child,
        }
    }
}

impl<T: 'static, A: AccessorShared<(), T>, W: ViewWidget<T>> SingleView<T, A, W> {
    /// Update data
    ///
    /// Other widgets with a view of this data are notified of the update.
    pub fn update(&mut self, mgr: &mut Manager, data: T) {
        self.accessor.set((), data);
        if let Some(handle) = self.accessor.update_handle() {
            mgr.trigger_update(handle, 0);
        }
    }
}

impl<T: 'static, A: Accessor<(), T>, W: ViewWidget<T>> WidgetConfig for SingleView<T, A, W> {
    fn configure(&mut self, mgr: &mut Manager) {
        if let Some(handle) = self.accessor.update_handle() {
            mgr.update_on_handle(handle, self.id());
        }
    }
}

impl<T: 'static, A: Accessor<(), T>, W: ViewWidget<T>> fmt::Debug for SingleView<T, A, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, accessor: {:?}, child: {:?} }}",
            self.core, self.accessor, self.child,
        )
    }
}
