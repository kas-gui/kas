// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! IntoWidget

use super::Widget;
use crate::hidden::{Discard, StrLabel};

pub trait IntoWidget<Data> {
    type Widget: Widget<Data = Data>;
    fn into_widget(self) -> Self::Widget;
}
impl<W: Widget> IntoWidget<W::Data> for W {
    type Widget = Self;
    fn into_widget(self) -> Self {
        self
    }
}
impl<Data> IntoWidget<Data> for &'static str {
    type Widget = Discard<Data, StrLabel>;
    fn into_widget(self) -> Self::Widget {
        Discard::new(StrLabel::new(self))
    }
}
