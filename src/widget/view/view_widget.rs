// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets
//!
//! View widgets exist as a view over some shared data.

use kas::prelude::*;
use kas::widget::*;
use std::fmt::Debug;
use std::marker::PhantomData;

/// View widget constructor
///
/// Types implementing this trait are able to construct a view widget for data
/// of type `T`. Several existing implementations are available...
pub trait View<T>: Debug + 'static {
    type Widget: Widget;

    /// Construct a default instance (with no data)
    fn default(&self) -> Self::Widget
    where
        T: Default;
    /// Construct an instance from a data value
    fn new(&self, data: T) -> Self::Widget;
    /// Set the viewed data
    fn set(&self, widget: &mut Self::Widget, data: T) -> TkAction;
}

/// Default view widget constructor
///
/// This struct implements [`View`], using a default widget for the data type.
#[derive(Clone, Debug, Default)]
pub struct DefaultView;

macro_rules! impl_via_to_string {
    ($t:ty) => {
        impl View<$t> for DefaultView {
            type Widget = Label<String>;
            fn default(&self) -> Self::Widget where $t: Default {
                Label::new("".to_string())
            }
            fn new(&self, data: $t) -> Self::Widget {
                Label::new(data.to_string())
            }
            fn set(&self, widget: &mut Self::Widget, data: $t) -> TkAction {
                widget.set_string(data.to_string())
            }
        }
    };
    ($t:ty, $($tt:ty),+) => {
        impl_via_to_string!($t);
        impl_via_to_string!($($tt),+);
    };
}
impl_via_to_string!(String, &'static str);
impl_via_to_string!(i8, i16, i32, i64, i128, isize);
impl_via_to_string!(u8, u16, u32, u64, u128, usize);
impl_via_to_string!(f32, f64);

impl View<bool> for DefaultView {
    type Widget = CheckBoxBare<VoidMsg>;
    fn default(&self) -> Self::Widget {
        CheckBoxBare::new()
    }
    fn new(&self, data: bool) -> Self::Widget {
        CheckBoxBare::new().with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// Custom view widget constructor
///
/// This struct implements [`View`], using a the parametrised widget type.
/// This struct is only usable where no extra data (such as a label) is required.
#[derive(Debug)]
pub struct CustomView<W: Widget> {
    _pd: PhantomData<W>,
}
impl<W: Widget> Clone for CustomView<W> {
    fn clone(&self) -> Self {
        Default::default()
    }
}
impl<W: Widget> Default for CustomView<W> {
    fn default() -> Self {
        CustomView {
            _pd: Default::default(),
        }
    }
}

impl<T> View<T> for CustomView<<DefaultView as View<T>>::Widget>
where
    DefaultView: View<T>,
{
    type Widget = <DefaultView as View<T>>::Widget;
    fn default(&self) -> Self::Widget
    where
        T: Default,
    {
        DefaultView.default()
    }
    fn new(&self, data: T) -> Self::Widget {
        DefaultView.new(data)
    }
    fn set(&self, widget: &mut Self::Widget, data: T) -> TkAction {
        DefaultView.set(widget, data)
    }
}

impl<G: EditGuard + Default> View<String> for CustomView<EditField<G>> {
    type Widget = EditField<G>;
    fn default(&self) -> Self::Widget {
        self.new("".to_string())
    }
    fn new(&self, data: String) -> Self::Widget {
        let guard = G::default();
        EditField::new(data).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, data: String) -> TkAction {
        widget.set_string(data)
    }
}
impl<G: EditGuard + Default> View<String> for CustomView<EditBox<G>> {
    type Widget = EditBox<G>;
    fn default(&self) -> Self::Widget {
        self.new("".to_string())
    }
    fn new(&self, data: String) -> Self::Widget {
        let guard = G::default();
        EditBox::new(data).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, data: String) -> TkAction {
        widget.set_string(data)
    }
}

impl<D: Directional + Default> View<f32> for CustomView<ProgressBar<D>> {
    type Widget = ProgressBar<D>;
    fn default(&self) -> Self::Widget {
        ProgressBar::new()
    }
    fn new(&self, data: f32) -> Self::Widget {
        ProgressBar::new().with_value(data)
    }
    fn set(&self, widget: &mut Self::Widget, data: f32) -> TkAction {
        widget.set_value(data)
    }
}

