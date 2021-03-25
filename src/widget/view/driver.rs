// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View drivers
//!
//! Intended usage is to import the module name rather than its contents, thus
//! allowing referal to e.g. `driver::Default`.

use kas::prelude::*;
use kas::widget::{self, *};
use std::fmt::Debug;
use std::marker::PhantomData;

/// View widget driver/binder
///
/// The controller binds data items with view widgets.
///
/// Note that the key type is not made available since in most cases view
/// widgets are not dependent on the element key. In rare cases where the key
/// is needed, it must be added to the data's `Item` type (see `data-list-view`
/// example).
///
/// Several existing implementations are available, most notably:
///
/// -   [`Default`](struct@Default) will choose a sensible widget to view the data
/// -   [`DefaultNav`] will choose a sensible widget to view the data
pub trait Driver<T>: Debug + 'static {
    /// Type of message sent by the widget
    type Msg;
    /// Type of the widget used to view data
    type Widget: kas::Widget<Msg = Self::Msg>;

    /// Construct a new instance with no data
    ///
    /// Such instances are used for sizing and cached widgets, but not shown.
    /// The controller may later call [`Driver::set`] on the widget then show it.
    fn new(&self) -> Self::Widget;
    /// Set the viewed data
    fn set(&self, widget: &mut Self::Widget, data: T) -> TkAction;
}

/// Default view widget constructor
///
/// This struct implements [`Driver`], using a default widget for the data type:
///
/// -   [`widget::Label`] for `String`, `&str`, integer and float types
/// -   [`widget::CheckBoxBare`] (disabled) for the bool type
#[derive(Clone, Debug, Default)]
pub struct Default;

/// Default view widget constructor supporting keyboard navigation
///
/// This struct implements [`Driver`], using a default widget for the data type
/// which also supports keyboard navigation:
///
/// -   [`widget::NavFrame`] around a [`widget::Label`] for `String`, `&str`,
///     integer and float types
/// -   [`widget::CheckBoxBare`] (disabled) for the bool type
#[derive(Clone, Debug, Default)]
pub struct DefaultNav;

macro_rules! impl_via_to_string {
    ($t:ty) => {
        impl Driver<$t> for Default {
            type Msg = VoidMsg;
            type Widget = Label<String>;
            fn new(&self) -> Self::Widget where $t: std::default::Default {
                Label::new("".to_string())
            }
            fn set(&self, widget: &mut Self::Widget, data: $t) -> TkAction {
                widget.set_string(data.to_string())
            }
        }
        impl Driver<$t> for DefaultNav {
            type Msg = VoidMsg;
            type Widget = NavFrame<Label<String>>;
            fn new(&self) -> Self::Widget where $t: std::default::Default {
                NavFrame::new(Label::new("".to_string()))
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

impl Driver<bool> for Default {
    type Msg = VoidMsg;
    type Widget = CheckBoxBare<VoidMsg>;
    fn new(&self) -> Self::Widget {
        CheckBoxBare::new().with_disabled(true)
    }
    fn set(&self, widget: &mut Self::Widget, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

impl Driver<bool> for DefaultNav {
    type Msg = VoidMsg;
    type Widget = CheckBoxBare<VoidMsg>;
    fn new(&self) -> Self::Widget {
        CheckBoxBare::new().with_disabled(true)
    }
    fn set(&self, widget: &mut Self::Widget, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// Custom view widget constructor
///
/// This struct implements [`Driver`], using a the parametrised widget type.
/// This struct is only usable where no extra data (such as a label) is required.
#[derive(Debug)]
pub struct Widget<W: kas::Widget> {
    _pd: PhantomData<W>,
}
impl<W: kas::Widget> Clone for Widget<W> {
    fn clone(&self) -> Self {
        std::default::Default::default()
    }
}
impl<W: kas::Widget> std::default::Default for Widget<W> {
    fn default() -> Self {
        Widget {
            _pd: PhantomData::default(),
        }
    }
}

impl<T> Driver<T> for Widget<<Default as Driver<T>>::Widget>
where
    Default: Driver<T>,
{
    type Msg = <Default as Driver<T>>::Msg;
    type Widget = <Default as Driver<T>>::Widget;
    fn new(&self) -> Self::Widget {
        Default.new()
    }
    fn set(&self, widget: &mut Self::Widget, data: T) -> TkAction {
        Default.set(widget, data)
    }
}

impl<G: EditGuard + std::default::Default> Driver<String> for Widget<EditField<G>> {
    type Msg = G::Msg;
    type Widget = EditField<G>;
    fn new(&self) -> Self::Widget {
        let guard = G::default();
        EditField::new("".to_string()).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, data: String) -> TkAction {
        widget.set_string(data)
    }
}
impl<G: EditGuard + std::default::Default> Driver<String> for Widget<EditBox<G>> {
    type Msg = G::Msg;
    type Widget = EditBox<G>;
    fn new(&self) -> Self::Widget {
        let guard = G::default();
        EditBox::new("".to_string()).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, data: String) -> TkAction {
        widget.set_string(data)
    }
}

impl<D: Directional + std::default::Default> Driver<f32> for Widget<ProgressBar<D>> {
    type Msg = VoidMsg;
    type Widget = ProgressBar<D>;
    fn new(&self) -> Self::Widget {
        ProgressBar::new()
    }
    fn set(&self, widget: &mut Self::Widget, data: f32) -> TkAction {
        widget.set_value(data)
    }
}

/// [`widget::CheckBox`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct CheckBox {
    label: AccelString,
}
impl CheckBox {
    /// Construct, with given `label`
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        let label = label.into();
        CheckBox { label }
    }
}
impl Driver<bool> for CheckBox {
    type Msg = bool;
    type Widget = widget::CheckBox<bool>;
    fn new(&self) -> Self::Widget {
        widget::CheckBox::new(self.label.clone()).on_toggle(|_, state| Some(state))
    }
    fn set(&self, widget: &mut Self::Widget, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// [`widget::RadioBoxBare`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct RadioBoxBare {
    handle: UpdateHandle,
}
impl RadioBoxBare {
    /// Construct, with given `handle`
    pub fn new(handle: UpdateHandle) -> Self {
        RadioBoxBare { handle }
    }
}
impl Driver<bool> for RadioBoxBare {
    type Msg = bool;
    type Widget = widget::RadioBoxBare<bool>;
    fn new(&self) -> Self::Widget {
        widget::RadioBoxBare::new(self.handle).on_select(|_| Some(true))
    }
    fn set(&self, widget: &mut Self::Widget, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// [`widget::RadioBox`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct RadioBox {
    label: AccelString,
    handle: UpdateHandle,
}
impl RadioBox {
    /// Construct, with given `label` and `handle`
    pub fn new<T: Into<AccelString>>(label: T, handle: UpdateHandle) -> Self {
        let label = label.into();
        RadioBox { label, handle }
    }
}
impl Driver<bool> for RadioBox {
    type Msg = bool;
    type Widget = widget::RadioBox<bool>;
    fn new(&self) -> Self::Widget {
        widget::RadioBox::new(self.label.clone(), self.handle).on_select(|_| Some(true))
    }
    fn set(&self, widget: &mut Self::Widget, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// [`widget::Slider`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct Slider<T: SliderType, D: Directional> {
    min: T,
    max: T,
    step: T,
    direction: D,
}
impl<T: SliderType, D: Directional + std::default::Default> Slider<T, D> {
    /// Construct, with given `min`, `max` and `step` (see [`Slider::new`])
    pub fn new(min: T, max: T, step: T) -> Self {
        Slider {
            min,
            max,
            step,
            direction: D::default(),
        }
    }
}
impl<T: SliderType, D: Directional> Slider<T, D> {
    /// Construct, with given `min`, `max`, `step` and `direction` (see [`Slider::new_with_direction`])
    pub fn new_with_direction(min: T, max: T, step: T, direction: D) -> Self {
        Slider {
            min,
            max,
            step,
            direction,
        }
    }
}
impl<T: SliderType, D: Directional> Driver<T> for Slider<T, D> {
    type Msg = T;
    type Widget = widget::Slider<T, D>;
    fn new(&self) -> Self::Widget {
        widget::Slider::new_with_direction(self.min, self.max, self.step, self.direction)
    }
    fn set(&self, widget: &mut Self::Widget, data: T) -> TkAction {
        widget.set_value(data)
    }
}
