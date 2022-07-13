// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View drivers
//!
//! Intended usage is to import the module name rather than its contents, thus
//! allowing referal to e.g. `driver::DefaultView`.

mod config;

use crate::{
    CheckBox, EditBox, EditField, EditGuard, Label, NavFrame, ProgressBar, RadioGroup, SliderValue,
    SpinnerValue,
};
use kas::model::SharedData;
use kas::prelude::*;
use std::default::Default;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::RangeInclusive;

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
/// -   [`DefaultView`] will choose a sensible widget to view the data
/// -   [`DefaultNav`] will choose a sensible widget to view the data
///
/// NOTE: `Item` is a direct type parameter (in addition to an assoc. type
/// param. of `SharedData`) only to avoid "conflicting implementations" errors.
/// Similar to: rust#20400, rust#92894. Given fixes, we may remove the param.
pub trait Driver<Item, Data: SharedData<Item = Item>>: Debug {
    /// Type of the widget used to view data
    type Widget: kas::Widget;

    /// Construct a new widget with no data
    ///
    /// Such instances are used for sizing and cached widgets, but not shown.
    /// The controller may later call [`Driver::set`] on the widget then show it.
    fn make(&self) -> Self::Widget;

    /// Set the viewed data
    ///
    /// The widget may expect `configure` to be called at least once before data
    /// is set and to have `set_rect` called after each time data is set.
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction;
}

/// Default view widget constructor
///
/// This struct implements [`Driver`], using a default widget for the data type:
///
/// -   [`crate::Label`] for `String`, `&str`, integer and float types
/// -   [`crate::CheckBox`] (disabled) for the bool type
#[derive(Clone, Debug, Default)]
pub struct DefaultView;

/// Default view widget constructor supporting keyboard navigation
///
/// This struct implements [`Driver`], using a default widget for the data type
/// which also supports keyboard navigation:
///
/// -   [`crate::NavFrame`] around a [`crate::Label`] for `String`, `&str`,
///     integer and float types
/// -   [`crate::CheckBox`] (disabled) for the bool type
#[derive(Clone, Debug, Default)]
pub struct DefaultNav;

macro_rules! impl_via_to_string {
    ($t:ty) => {
        impl<Data: SharedData<Item = $t>> Driver<$t, Data> for DefaultView {
            type Widget = Label<String>;
            fn make(&self) -> Self::Widget {
                Label::new("".to_string())
            }
            fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
                data.get_cloned(key).map(|item| widget.set_string(item.to_string())).unwrap_or(TkAction::EMPTY)
            }
        }
        impl<Data: SharedData<Item = $t>> Driver<$t, Data> for DefaultNav {
            type Widget = NavFrame<Label<String>>;
            fn make(&self) -> Self::Widget {
                NavFrame::new(Label::new("".to_string()))
            }
            fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
                data.get_cloned(key).map(|item| widget.set_string(item.to_string())).unwrap_or(TkAction::EMPTY)
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

impl<Data: SharedData<Item = bool>> Driver<bool, Data> for DefaultView {
    type Widget = CheckBox;
    fn make(&self) -> Self::Widget {
        CheckBox::new_on(|mgr, state| mgr.push_msg(state)).with_editable(false)
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_bool(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

impl<Data: SharedData<Item = bool>> Driver<bool, Data> for DefaultNav {
    type Widget = CheckBox;
    fn make(&self) -> Self::Widget {
        CheckBox::new().with_editable(false)
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_bool(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

/// Custom view widget constructor
///
/// This struct implements [`Driver`], using a the parametrised widget type.
/// This struct is only usable where no extra data (such as a label) is required.
#[derive(Clone, Debug)]
pub struct Widget<W: kas::Widget> {
    _pd: PhantomData<W>,
}
impl<W: kas::Widget> Default for Widget<W> {
    fn default() -> Self {
        Widget {
            _pd: PhantomData::default(),
        }
    }
}

// NOTE: this implementation conflicts, where it did not before adding the Data
// type parameter to Driver. Possibly it can be re-enabled in the future.
/*
impl<Item, Data: SharedData<Item = Item>> Driver<Item, Data>
    for Widget<<DefaultView as Driver<Item, Data>>::Widget>
where
    DefaultView: Driver<Item, Data>,
{
    type Widget = <DefaultView as Driver<Item, Data>>::Widget;
    fn make(&self) -> Self::Widget {
        DefaultView.make()
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        DefaultView.set(widget, data, key)
    }
}*/

impl<G: EditGuard + Default, Data: SharedData<Item = String>> Driver<String, Data>
    for Widget<EditField<G>>
{
    type Widget = EditField<G>;
    fn make(&self) -> Self::Widget {
        let guard = G::default();
        EditField::new("".to_string()).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_string(item))
            .unwrap_or(TkAction::EMPTY)
    }
}
impl<G: EditGuard + Default, Data: SharedData<Item = String>> Driver<String, Data>
    for Widget<EditBox<G>>
{
    type Widget = EditBox<G>;
    fn make(&self) -> Self::Widget {
        let guard = G::default();
        EditBox::new("".to_string()).with_guard(guard)
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_string(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

impl<D: Directional + Default, Data: SharedData<Item = f32>> Driver<f32, Data>
    for Widget<ProgressBar<D>>
{
    type Widget = ProgressBar<D>;
    fn make(&self) -> Self::Widget {
        ProgressBar::new()
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_value(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

/// [`crate::CheckButton`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct CheckButton {
    label: AccelString,
}
impl CheckButton {
    /// Construct, with given `label`
    pub fn make<T: Into<AccelString>>(label: T) -> Self {
        let label = label.into();
        CheckButton { label }
    }
}
impl<Data: SharedData<Item = bool>> Driver<bool, Data> for CheckButton {
    type Widget = crate::CheckButton;
    fn make(&self) -> Self::Widget {
        crate::CheckButton::new(self.label.clone()).on_toggle(|mgr, state| mgr.push_msg(state))
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_bool(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

/// [`crate::RadioBox`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct RadioBox {
    group: RadioGroup,
}
impl RadioBox {
    /// Construct, with given `group`
    pub fn make(group: RadioGroup) -> Self {
        RadioBox { group }
    }
}
impl<Data: SharedData<Item = bool>> Driver<bool, Data> for RadioBox {
    type Widget = crate::RadioBox;
    fn make(&self) -> Self::Widget {
        crate::RadioBox::new(self.group.clone()).on_select(|mgr| mgr.push_msg(true))
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_bool(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

/// [`crate::RadioButton`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct RadioButton {
    label: AccelString,
    group: RadioGroup,
}
impl RadioButton {
    /// Construct, with given `label` and `group`
    pub fn make<T: Into<AccelString>>(label: T, group: RadioGroup) -> Self {
        let label = label.into();
        RadioButton { label, group }
    }
}
impl<Data: SharedData<Item = bool>> Driver<bool, Data> for RadioButton {
    type Widget = crate::RadioButton;
    fn make(&self) -> Self::Widget {
        crate::RadioButton::new(self.label.clone(), self.group.clone())
            .on_select(|mgr| mgr.push_msg(true))
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_bool(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

/// [`crate::Slider`] view widget constructor
#[derive(Clone, Debug)]
pub struct Slider<T: SliderValue, D: Directional> {
    range: RangeInclusive<T>,
    step: T,
    direction: D,
}
impl<T: SliderValue, D: Directional + Default> Slider<T, D> {
    /// Construct, with given `range` and `step` (see [`crate::Slider::new`])
    pub fn make(range: RangeInclusive<T>, step: T) -> Self {
        Slider {
            range,
            step,
            direction: D::default(),
        }
    }
}
impl<T: SliderValue, D: Directional> Slider<T, D> {
    /// Construct, with given `range`, `step` and `direction` (see [`Slider::new_with_direction`])
    pub fn new_with_direction(range: RangeInclusive<T>, step: T, direction: D) -> Self {
        Slider {
            range,
            step,
            direction,
        }
    }
}
impl<D: Directional, Data: SharedData> Driver<Data::Item, Data> for Slider<Data::Item, D>
where
    Data::Item: SliderValue,
{
    type Widget = crate::Slider<Data::Item, D>;
    fn make(&self) -> Self::Widget {
        crate::Slider::new_with_direction(self.range.clone(), self.step, self.direction)
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_value(item))
            .unwrap_or(TkAction::EMPTY)
    }
}

/// [`crate::Spinner`] view widget constructor
#[derive(Clone, Debug)]
pub struct Spinner<T: SpinnerValue> {
    range: RangeInclusive<T>,
    step: T,
}
impl<T: SpinnerValue + Default> Spinner<T> {
    /// Construct, with given `range` and `step` (see [`crate::Spinner::new`])
    pub fn make(range: RangeInclusive<T>, step: T) -> Self {
        Spinner { range, step }
    }
}
impl<Data: SharedData> Driver<Data::Item, Data> for Spinner<Data::Item>
where
    Data::Item: SpinnerValue,
{
    type Widget = crate::Spinner<Data::Item>;
    fn make(&self) -> Self::Widget {
        crate::Spinner::new(self.range.clone(), self.step)
    }
    fn set(&self, widget: &mut Self::Widget, data: &Data, key: &Data::Key) -> TkAction {
        data.get_cloned(key)
            .map(|item| widget.set_value(item))
            .unwrap_or(TkAction::EMPTY)
    }
}
