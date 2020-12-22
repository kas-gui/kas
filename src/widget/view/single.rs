// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{Accessor, DefaultView, ViewWidget};
use kas::prelude::*;
use std::fmt;
use std::marker::PhantomData;

/// Single view widget
#[derive(Clone, Default, Widget)]
#[layout(single)]
#[handler(msg=<W as Handler>::Msg)]
pub struct SingleView<T: 'static, A: Accessor<(), T>, W: ViewWidget<T> = <T as DefaultView>::Widget>
{
    #[widget_core]
    core: CoreData,
    _t: PhantomData<T>,
    data: A,
    #[widget]
    child: W,
}

impl<T: 'static, A: Accessor<(), T>, W: ViewWidget<T>> SingleView<T, A, W> {
    /// Construct a new instance
    pub fn new(data: A) -> Self {
        let child = W::new(data.get(()));
        SingleView {
            core: Default::default(),
            _t: Default::default(),
            data,
            child,
        }
    }
}

impl<T: 'static, A: Accessor<(), T>, W: ViewWidget<T>> fmt::Debug for SingleView<T, A, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SingleView {{ core: {:?}, data: {:?}, child: {:?} }}",
            self.core, self.data, self.child,
        )
    }
}
