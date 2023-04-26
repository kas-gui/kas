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

pub trait IntoVecWidget<Data> {
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>>;
}
impl<Data, W> IntoVecWidget<Data> for &[W]
where
    for<'a> &'a W: IntoWidget<Data>,
    for<'a> <&'a W as IntoWidget<Data>>::Widget: 'static,
{
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>> {
        let mut v: Vec<Box<dyn Widget<Data = Data>>> = Vec::with_capacity(self.len());
        for x in self.iter() {
            v.push(Box::new(x.into_widget()));
        }
        v
    }
}
impl<Data> IntoVecWidget<Data> for () {
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>> {
        vec![]
    }
}
impl<Data, W1> IntoVecWidget<Data> for (W1,)
where
    W1: IntoWidget<Data>,
    <W1 as IntoWidget<Data>>::Widget: 'static,
{
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>> {
        vec![Box::new(self.0.into_widget())]
    }
}
impl<Data, W1, W2> IntoVecWidget<Data> for (W1, W2)
where
    W1: IntoWidget<Data>,
    <W1 as IntoWidget<Data>>::Widget: 'static,
    W2: IntoWidget<Data>,
    <W2 as IntoWidget<Data>>::Widget: 'static,
{
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>> {
        vec![
            Box::new(self.0.into_widget()),
            Box::new(self.1.into_widget()),
        ]
    }
}
impl<Data, W1, W2, W3, W4> IntoVecWidget<Data> for (W1, W2, W3, W4)
where
    W1: IntoWidget<Data>,
    <W1 as IntoWidget<Data>>::Widget: 'static,
    W2: IntoWidget<Data>,
    <W2 as IntoWidget<Data>>::Widget: 'static,
    W3: IntoWidget<Data>,
    <W3 as IntoWidget<Data>>::Widget: 'static,
    W4: IntoWidget<Data>,
    <W4 as IntoWidget<Data>>::Widget: 'static,
{
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>> {
        vec![
            Box::new(self.0.into_widget()),
            Box::new(self.1.into_widget()),
            Box::new(self.2.into_widget()),
            Box::new(self.3.into_widget()),
        ]
    }
}
impl<Data, W1, W2, W3, W4, W5> IntoVecWidget<Data> for (W1, W2, W3, W4, W5)
where
    W1: IntoWidget<Data>,
    <W1 as IntoWidget<Data>>::Widget: 'static,
    W2: IntoWidget<Data>,
    <W2 as IntoWidget<Data>>::Widget: 'static,
    W3: IntoWidget<Data>,
    <W3 as IntoWidget<Data>>::Widget: 'static,
    W4: IntoWidget<Data>,
    <W4 as IntoWidget<Data>>::Widget: 'static,
    W5: IntoWidget<Data>,
    <W5 as IntoWidget<Data>>::Widget: 'static,
{
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>> {
        vec![
            Box::new(self.0.into_widget()),
            Box::new(self.1.into_widget()),
            Box::new(self.2.into_widget()),
            Box::new(self.3.into_widget()),
            Box::new(self.4.into_widget()),
        ]
    }
}
impl<Data, W1, W2, W3, W4, W5, W6> IntoVecWidget<Data> for (W1, W2, W3, W4, W5, W6)
where
    W1: IntoWidget<Data>,
    <W1 as IntoWidget<Data>>::Widget: 'static,
    W2: IntoWidget<Data>,
    <W2 as IntoWidget<Data>>::Widget: 'static,
    W3: IntoWidget<Data>,
    <W3 as IntoWidget<Data>>::Widget: 'static,
    W4: IntoWidget<Data>,
    <W4 as IntoWidget<Data>>::Widget: 'static,
    W5: IntoWidget<Data>,
    <W5 as IntoWidget<Data>>::Widget: 'static,
    W6: IntoWidget<Data>,
    <W6 as IntoWidget<Data>>::Widget: 'static,
{
    fn into_vec_widget(self) -> Vec<Box<dyn Widget<Data = Data>>> {
        vec![
            Box::new(self.0.into_widget()),
            Box::new(self.1.into_widget()),
            Box::new(self.2.into_widget()),
            Box::new(self.3.into_widget()),
            Box::new(self.4.into_widget()),
            Box::new(self.5.into_widget()),
        ]
    }
}
