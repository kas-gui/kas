// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View drivers
//!
//! Intended usage is to import the module name rather than its contents, thus
//! allowing referal to e.g. `driver::Default`.

use kas::event::UpdateHandle;
use kas::widget::{self, *};
use kas::prelude::*;
use std::fmt::Debug;
use std::marker::PhantomData;

/// View widget controller
///
/// The controller binds data items with view widgets.
///
/// Several existing implementations are available, most notably:
///
/// -   [`Default`](struct@Default) will choose a sensible widget to view the data
pub trait Driver<K, T>: Debug + 'static {
    /// Type of the widget used to view data
    type Widget: kas::Widget;

    /// Construct a default instance (with no data)
    ///
    /// Such instances are used for sizing and cached widgets, but not shown.
    /// The controller may later call [`Driver::set`] on the widget then show it.
    fn default(&self) -> Self::Widget;
    /// Construct an instance from a data value
    fn new(&self, key: K, data: T) -> Self::Widget;
    /// Set the viewed data
    fn set(&self, widget: &mut Self::Widget, key: K, data: T) -> TkAction;
    /// Get data from the view
    ///
    /// "View" widgets which allow the user to manipulate their data (e.g. a
    /// slider or edit box) should return a copy of that data here; other
    /// widgets should just return `None`.
    ///
    /// When a view widget emits [`Response::Msg`], this method is called to
    /// update the shared data set with the returned value (if any).
    fn get(&self, widget: &Self::Widget, key: &K) -> Option<T>;
}

/// Default view widget constructor
///
/// This struct implements [`Driver`], using a default widget for the data type:
///
/// -   [`widget::Label`] for `String`, `&str`, integer and float types
/// -   [`widget::CheckBoxBare`] (disabled) for the bool type
#[derive(Clone, Debug, Default)]
pub struct Default;

macro_rules! impl_via_to_string {
    ($t:ty) => {
        impl<K> Driver<K, $t> for Default {
            type Widget = Label<String>;
            fn default(&self) -> Self::Widget where $t: std::default::Default {
                Label::new("".to_string())
            }
            fn new(&self, _: K, data: $t) -> Self::Widget {
                Label::new(data.to_string())
            }
            fn set(&self, widget: &mut Self::Widget, _: K, data: $t) -> TkAction {
                widget.set_string(data.to_string())
            }
            fn get(&self, _: &Self::Widget, _: &K) -> Option<$t> { None }
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

impl<K> Driver<K, bool> for Default {
    type Widget = CheckBoxBare<VoidMsg>;
    fn default(&self) -> Self::Widget {
        CheckBoxBare::new().with_disabled(true)
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        CheckBoxBare::new().with_state(data).with_disabled(true)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
    fn get(&self, widget: &Self::Widget, _: &K) -> Option<bool> {
        Some(widget.get_bool())
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

// TODO: we would like to enable this impl, but can't (since adding K parameter)
// due to conflicting impls (coherence issue — rust#19032).
// impl<K, T> Driver<K, T> for Widget<<Default as Driver<K, T>>::Widget>
// where
//     Default: Driver<K, T>,
// {
//     type Widget = <Default as Driver<K, T>>::Widget;
//     fn default(&self) -> Self::Widget {
//         Default.default()
//     }
//     fn new(&self, key: K, data: T) -> Self::Widget {
//         Default.new(key, data)
//     }
//     fn set(&self, widget: &mut Self::Widget, key: K, data: T) -> TkAction {
//         Default.set(widget, key, data)
//     }
//     fn get(&self, widget: &Self::Widget, key: K) -> Option<T> {
//         Some(Default.set(widget, key, data))
//     }
// }

impl<K, G: EditGuard + std::default::Default> Driver<K, String> for Widget<EditField<G>> {
    type Widget = EditField<G>;
    fn default(&self) -> Self::Widget {
        let guard = G::default();
        EditField::new("".to_string()).with_guard(guard)
    }
    fn new(&self, _: K, data: String) -> Self::Widget {
        let guard = G::default();
        EditField::new(data).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: String) -> TkAction {
        widget.set_string(data)
    }
    fn get(&self, widget: &Self::Widget, _: &K) -> Option<String> {
        Some(widget.get_string())
    }
}
impl<K, G: EditGuard + std::default::Default> Driver<K, String> for Widget<EditBox<G>> {
    type Widget = EditBox<G>;
    fn default(&self) -> Self::Widget {
        let guard = G::default();
        EditBox::new("".to_string()).with_guard(guard)
    }
    fn new(&self, _: K, data: String) -> Self::Widget {
        let guard = G::default();
        EditBox::new(data).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: String) -> TkAction {
        widget.set_string(data)
    }
    fn get(&self, widget: &Self::Widget, _: &K) -> Option<String> {
        Some(widget.get_string())
    }
}

impl<K, D: Directional + std::default::Default> Driver<K, f32> for Widget<ProgressBar<D>> {
    type Widget = ProgressBar<D>;
    fn default(&self) -> Self::Widget {
        ProgressBar::new()
    }
    fn new(&self, _: K, data: f32) -> Self::Widget {
        ProgressBar::new().with_value(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: f32) -> TkAction {
        widget.set_value(data)
    }
    fn get(&self, _: &Self::Widget, _: &K) -> Option<f32> {
        None
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
impl<K> Driver<K, bool> for CheckBox {
    type Widget = widget::CheckBox<VoidMsg>;
    fn default(&self) -> Self::Widget {
        widget::CheckBox::new(self.label.clone())
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        <Self as Driver<K, bool>>::default(self).with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
    fn get(&self, widget: &Self::Widget, _: &K) -> Option<bool> {
        Some(widget.get_bool())
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
impl<K> Driver<K, bool> for RadioBoxBare {
    type Widget = widget::RadioBoxBare<VoidMsg>;
    fn default(&self) -> Self::Widget {
        widget::RadioBoxBare::new(self.handle)
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        <Self as Driver<K, bool>>::default(self).with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
    fn get(&self, widget: &Self::Widget, _: &K) -> Option<bool> {
        Some(widget.get_bool())
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
impl<K> Driver<K, bool> for RadioBox {
    type Widget = widget::RadioBox<VoidMsg>;
    fn default(&self) -> Self::Widget {
        widget::RadioBox::new(self.label.clone(), self.handle)
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        <Self as Driver<K, bool>>::default(self).with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
    fn get(&self, widget: &Self::Widget, _: &K) -> Option<bool> {
        Some(widget.get_bool())
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
impl<K, T: SliderType, D: Directional> Driver<K, T> for Slider<T, D> {
    type Widget = widget::Slider<T, D>;
    fn default(&self) -> Self::Widget {
        widget::Slider::new_with_direction(self.min, self.max, self.step, self.direction)
    }
    fn new(&self, _: K, data: T) -> Self::Widget {
        widget::Slider::new_with_direction(self.min, self.max, self.step, self.direction)
            .with_value(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: T) -> TkAction {
        widget.set_value(data)
    }
    fn get(&self, widget: &Self::Widget, _: &K) -> Option<T> {
        Some(widget.value())
    }
}
