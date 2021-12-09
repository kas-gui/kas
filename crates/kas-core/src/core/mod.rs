// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Core widget types

mod data;
mod impls;
mod widget;
mod widget_id;

pub use data::*;
pub use widget::*;
pub use widget_id::WidgetId;

/// Provides a convenient `.boxed()` method on implementors
pub trait Boxed<T: ?Sized> {
    /// Boxing method
    fn boxed(self) -> Box<T>;
}

impl<W: Widget + Sized> Boxed<dyn Widget<Msg = W::Msg>> for W {
    fn boxed(self) -> Box<dyn Widget<Msg = W::Msg>> {
        Box::new(self)
    }
}
