// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Single view widget

use super::{Accessor, DefaultView, ViewWidget};
use kas::prelude::*;

/// Single view widget
#[derive(Clone, Default, Debug, Widget)]
#[layout(single)]
#[handler(msg=<W as Handler>::Msg)]
pub struct SingleView<
    A: Accessor<()>,
    W: ViewWidget<A::Item> = <<A as Accessor<()>>::Item as DefaultView>::Widget,
> {
    first_id: WidgetId,
    #[widget_core]
    core: CoreData,
    data: A,
    #[widget]
    child: W,
}

impl<A: Accessor<()>, W: ViewWidget<A::Item>> SingleView<A, W> {
    /// Construct a new instance
    pub fn new(data: A) -> Self {
        let value = data.get(());
        SingleView {
            first_id: Default::default(),
            core: Default::default(),
            data,
            child: W::new(value),
        }
    }
}
