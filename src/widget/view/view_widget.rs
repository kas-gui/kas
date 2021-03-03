// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets
//!
//! View widgets exist as a view over some shared data.

use kas::event::UpdateHandle;
use kas::prelude::*;
use kas::widget::*;
use std::fmt::Debug;
use std::marker::PhantomData;

/// View widget constructor
///
/// Types implementing this trait are able to construct a view widget for data
/// of type `T`. Several existing implementations are available...
pub trait View<K, T>: Debug + 'static {
    type Widget: Widget;

    /// Construct a default instance (with no data)
    ///
    /// Such instances are used for sizing and cached widgets, but not shown.
    /// The controller may later call [`View::set`] on the widget then show it.
    fn default(&self) -> Self::Widget;
    /// Construct an instance from a data value
    fn new(&self, key: K, data: T) -> Self::Widget;
    /// Set the viewed data
    fn set(&self, widget: &mut Self::Widget, key: K, data: T) -> TkAction;
}

/// Default view widget constructor
///
/// This struct implements [`View`], using a default widget for the data type.
#[derive(Clone, Debug, Default)]
pub struct DefaultView;

macro_rules! impl_via_to_string {
    ($t:ty) => {
        impl<K> View<K, $t> for DefaultView {
            type Widget = Label<String>;
            fn default(&self) -> Self::Widget where $t: Default {
                Label::new("".to_string())
            }
            fn new(&self, _: K, data: $t) -> Self::Widget {
                Label::new(data.to_string())
            }
            fn set(&self, widget: &mut Self::Widget, _: K, data: $t) -> TkAction {
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

impl<K> View<K, bool> for DefaultView {
    type Widget = CheckBoxBare<VoidMsg>;
    fn default(&self) -> Self::Widget {
        CheckBoxBare::new()
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        CheckBoxBare::new().with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// Custom view widget constructor
///
/// This struct implements [`View`], using a the parametrised widget type.
/// This struct is only usable where no extra data (such as a label) is required.
#[derive(Debug)]
pub struct WidgetView<W: Widget> {
    _pd: PhantomData<W>,
}
impl<W: Widget> Clone for WidgetView<W> {
    fn clone(&self) -> Self {
        Default::default()
    }
}
impl<W: Widget> Default for WidgetView<W> {
    fn default() -> Self {
        WidgetView {
            _pd: Default::default(),
        }
    }
}

// TODO: we would like to enable this impl, but can't (since adding K parameter)
// due to conflicting impls (coherence issue — rust#19032).
// impl<K, T> View<K, T> for WidgetView<<DefaultView as View<K, T>>::Widget>
// where
//     DefaultView: View<K, T>,
// {
//     type Widget = <DefaultView as View<K, T>>::Widget;
//     fn default(&self) -> Self::Widget {
//         DefaultView.default()
//     }
//     fn new(&self, key: K, data: T) -> Self::Widget {
//         DefaultView.new(key, data)
//     }
//     fn set(&self, widget: &mut Self::Widget, key: K, data: T) -> TkAction {
//         DefaultView.set(widget, key, data)
//     }
// }

impl<K, G: EditGuard + Default> View<K, String> for WidgetView<EditField<G>> {
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
}
impl<K, G: EditGuard + Default> View<K, String> for WidgetView<EditBox<G>> {
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
}

impl<K, D: Directional + Default> View<K, f32> for WidgetView<ProgressBar<D>> {
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
}

/// [`CheckBox`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct CheckBoxView {
    label: AccelString,
}
impl CheckBoxView {
    /// Construct, with given `label`
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        let label = label.into();
        CheckBoxView { label }
    }
}
impl<K> View<K, bool> for CheckBoxView {
    type Widget = CheckBox<VoidMsg>;
    fn default(&self) -> Self::Widget {
        CheckBox::new(self.label.clone())
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        <Self as View<K, bool>>::default(self).with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// [`RadioBoxBare`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct RadioBoxBareView {
    handle: UpdateHandle,
}
impl RadioBoxBareView {
    /// Construct, with given `handle`
    pub fn new(handle: UpdateHandle) -> Self {
        RadioBoxBareView { handle }
    }
}
impl<K> View<K, bool> for RadioBoxBareView {
    type Widget = RadioBoxBare<VoidMsg>;
    fn default(&self) -> Self::Widget {
        RadioBoxBare::new(self.handle)
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        <Self as View<K, bool>>::default(self).with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// [`RadioBox`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct RadioBoxView {
    label: AccelString,
    handle: UpdateHandle,
}
impl RadioBoxView {
    /// Construct, with given `label` and `handle`
    pub fn new<T: Into<AccelString>>(label: T, handle: UpdateHandle) -> Self {
        let label = label.into();
        RadioBoxView { label, handle }
    }
}
impl<K> View<K, bool> for RadioBoxView {
    type Widget = RadioBox<VoidMsg>;
    fn default(&self) -> Self::Widget {
        RadioBox::new(self.label.clone(), self.handle)
    }
    fn new(&self, _: K, data: bool) -> Self::Widget {
        <Self as View<K, bool>>::default(self).with_state(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: bool) -> TkAction {
        widget.set_bool(data)
    }
}

/// [`Slider`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct SliderView<T: SliderType, D: Directional> {
    min: T,
    max: T,
    step: T,
    direction: D,
}
impl<T: SliderType, D: Directional + Default> SliderView<T, D> {
    /// Construct, with given `min`, `max` and `step` (see [`Slider::new`])
    pub fn new(min: T, max: T, step: T) -> Self {
        SliderView {
            min,
            max,
            step,
            direction: D::default(),
        }
    }
}
impl<T: SliderType, D: Directional> SliderView<T, D> {
    /// Construct, with given `min`, `max`, `step` and `direction` (see [`Slider::new_with_direction`])
    pub fn new_with_direction(min: T, max: T, step: T, direction: D) -> Self {
        SliderView {
            min,
            max,
            step,
            direction,
        }
    }
}
impl<K, T: SliderType, D: Directional> View<K, T> for SliderView<T, D> {
    type Widget = Slider<T, D>;
    fn default(&self) -> Self::Widget {
        Slider::new_with_direction(self.min, self.max, self.step, self.direction)
    }
    fn new(&self, _: K, data: T) -> Self::Widget {
        Slider::new_with_direction(self.min, self.max, self.step, self.direction).with_value(data)
    }
    fn set(&self, widget: &mut Self::Widget, _: K, data: T) -> TkAction {
        widget.set_value(data)
    }
}
