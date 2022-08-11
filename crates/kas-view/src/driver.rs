// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View drivers
//!
//! This module provides the [`Driver`] trait along with three classes of
//! implementations:
//!
//! -   [`View`], [`NavView`] provide "default views" over simple content
//! -   [`EventConfig`] provides an editor over a specific complex data type
//! -   Other types construct a (parametrized) input control over simple data
//!
//! Intended usage is to import the module name rather than its contents, thus
//! allowing referal to e.g. `driver::View`.

mod config;
pub use config::EventConfig;

use kas::model::SharedData;
use kas::prelude::*;
use kas::theme::TextClass;
use kas_widgets::edit::{EditGuard, GuardNotify};
use kas_widgets::{CheckBox, Label, NavFrame, RadioGroup, SliderValue, SpinnerValue};
use std::default::Default;
use std::fmt::Debug;
use std::ops::RangeInclusive;

/// View widget driver/binder
///
/// The driver can:
///
/// -   construct (empty) widgets with [`Self::make`]
/// -   assign data to an existing widget with [`Self::set`]
/// -   (optional) handle messages from a widget with [`Self::on_message`]
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
    fn set(&self, widget: &mut Self::Widget, key: &Data::Key, item: Item) -> TkAction;

    /// Handle a message from a widget
    ///
    /// This method is called when a view widget returns with a message; it
    /// may retrieve this message with [`EventMgr::try_pop_msg`].
    ///
    /// There are three main ways of implementing this method:
    ///
    /// 1.  Do nothing. This is always safe, though may result in unhandled
    ///     message warnings when the view widget is interactive.
    /// 2.  On user input actions, view widgets send a message including their
    ///     content (potentially wrapped with a user-defined enum or struct
    ///     type). The implementation of this method retrieves this message and
    ///     updates `data` given this content. In this case, the `widget`
    ///     parameter is not used.
    /// 3.  On user input actions, view widgets send a "trigger" message (likely
    ///     a unit struct). The implementation of this method retrieves this
    ///     message updates `data` using values read from `widget`.
    ///
    /// See, for example, the implementation for [`CheckButton`]: the `make`
    /// method assigns a state-change handler which `on_message` uses to update
    /// the shared data.
    ///
    /// Default implementation: do nothing.
    fn on_message(
        &self,
        mgr: &mut EventMgr,
        widget: &mut Self::Widget,
        data: &Data,
        key: &Data::Key,
    ) {
        let _ = (mgr, widget, data, key);
    }
}

/// Default view widget constructor
///
/// This struct implements [`Driver`], using a default widget for the data type:
///
/// -   [`kas_widgets::Label`] for `String`, `&str`, integer and float types
/// -   [`kas_widgets::CheckBox`] (read-only) for the bool type
#[derive(Clone, Copy, Debug, Default)]
pub struct View;

/// Default view widget constructor supporting keyboard navigation
///
/// This struct implements [`Driver`], using a default widget for the data type
/// which also supports keyboard navigation:
///
/// -   [`kas_widgets::NavFrame`] around a [`kas_widgets::Label`] for `String`, `&str`,
///     integer and float types
/// -   [`kas_widgets::CheckBox`] (read-only) for the bool type
#[derive(Clone, Copy, Debug, Default)]
pub struct NavView;

macro_rules! impl_via_to_string {
    ($t:ty) => {
        impl<Data: SharedData<Item = $t>> Driver<$t, Data> for View {
            type Widget = Label<String>;
            fn make(&self) -> Self::Widget {
                Label::new("".to_string())
            }
            fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: $t) -> TkAction {
                widget.set_string(item.to_string())
            }
        }
        impl<Data: SharedData<Item = $t>> Driver<$t, Data> for NavView {
            type Widget = NavFrame<Label<String>>;
            fn make(&self) -> Self::Widget {
                NavFrame::new(Label::new("".to_string()))
            }
            fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: $t) -> TkAction {
                widget.set_string(item.to_string())
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

impl<Data: SharedData<Item = bool>> Driver<bool, Data> for View {
    type Widget = CheckBox;
    fn make(&self) -> Self::Widget {
        CheckBox::new_on(|mgr, state| mgr.push_msg(state)).with_editable(false)
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: bool) -> TkAction {
        widget.set_bool(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(state) = mgr.try_pop_msg() {
            data.update(mgr, key, state);
        }
    }
}

impl<Data: SharedData<Item = bool>> Driver<bool, Data> for NavView {
    type Widget = CheckBox;
    fn make(&self) -> Self::Widget {
        CheckBox::new_on(|mgr, state| mgr.push_msg(state)).with_editable(false)
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: bool) -> TkAction {
        widget.set_bool(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(state) = mgr.try_pop_msg() {
            data.update(mgr, key, state);
        }
    }
}

impl_scope! {
    /// [`kas_widgets::EditField`] view widget constructor
    ///
    /// This is parameterized `G`: [`EditGuard`], which defaults to [`GuardNotify`].
    /// The guard should send a [`String`] message to enable updates on edit.
    #[impl_default(where G: Default)]
    #[derive(Clone, Copy, Debug)]
    pub struct EditField<G: EditGuard + Clone = GuardNotify> {
        guard: G,
        class: TextClass = TextClass::Edit(false),
    }
    impl Self where G: Default {
        /// Construct
        #[inline]
        pub fn new() -> Self {
            Self::default()
        }
    }
    impl Self {
        /// Construct with given `guard`
        #[inline]
        #[must_use]
        pub fn with_guard(mut self, guard: G) -> Self {
            self.guard = guard;
            self
        }

        /// Set whether the editor uses multi-line mode
        ///
        /// This replaces any class set by [`Self::with_class`].
        ///
        /// See: [`kas_widgets::EditField::with_multi_line`].
        #[inline]
        #[must_use]
        pub fn with_multi_line(self, multi_line: bool) -> Self {
            self.with_class(TextClass::Edit(multi_line))
        }

        /// Set the text class used
        ///
        /// See: [`kas_widgets::EditField::with_class`].
        #[inline]
        #[must_use]
        pub fn with_class(mut self, class: TextClass) -> Self {
            self.class = class;
            self
        }
    }
}
impl<G: EditGuard + Clone, Data: SharedData<Item = String>> Driver<String, Data> for EditField<G> {
    type Widget = kas_widgets::EditField<G>;
    fn make(&self) -> Self::Widget {
        kas_widgets::EditField::new("".to_string())
            .with_guard(self.guard.clone())
            .with_class(self.class)
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: String) -> TkAction {
        widget.set_string(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(item) = mgr.try_pop_msg() {
            data.update(mgr, key, item);
        }
    }
}

impl_scope! {
    /// [`kas_widgets::EditBox`] view widget constructor
    ///
    /// This is parameterized `G`: [`EditGuard`], which defaults to [`GuardNotify`].
    /// The guard should send a [`String`] message to enable updates on edit.
    #[impl_default(where G: Default)]
    #[derive(Clone, Copy, Debug)]
    pub struct EditBox<G: EditGuard + Clone = GuardNotify> {
        guard: G,
        class: TextClass = TextClass::Edit(false),
    }
    impl Self where G: Default {
        /// Construct
        #[inline]
        pub fn new() -> Self {
            Self::default()
        }
    }
    impl Self {
        /// Construct with given `guard`
        #[inline]
        #[must_use]
        pub fn with_guard(mut self, guard: G) -> Self {
            self.guard = guard;
            self
        }

        /// Set whether the editor uses multi-line mode
        ///
        /// This replaces any class set by [`Self::with_class`].
        ///
        /// See: [`kas_widgets::EditBox::with_multi_line`].
        #[inline]
        #[must_use]
        pub fn with_multi_line(self, multi_line: bool) -> Self {
            self.with_class(TextClass::Edit(multi_line))
        }

        /// Set the text class used
        ///
        /// See: [`kas_widgets::EditBox::with_class`].
        #[inline]
        #[must_use]
        pub fn with_class(mut self, class: TextClass) -> Self {
            self.class = class;
            self
        }
    }
}
impl<G: EditGuard + Clone, Data: SharedData<Item = String>> Driver<String, Data> for EditBox<G> {
    type Widget = kas_widgets::EditBox<G>;
    fn make(&self) -> Self::Widget {
        kas_widgets::EditBox::new("".to_string()).with_guard(self.guard.clone())
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: String) -> TkAction {
        widget.set_string(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(item) = mgr.try_pop_msg() {
            data.update(mgr, key, item);
        }
    }
}

/// [`kas_widgets::ProgressBar`] view widget constructor
#[derive(Clone, Copy, Debug, Default)]
pub struct ProgressBar<D: Directional> {
    direction: D,
}
impl<D: Directional + Default> ProgressBar<D> {
    /// Construct
    pub fn new() -> Self {
        ProgressBar::new_with_direction(Default::default())
    }
}
impl<D: Directional> ProgressBar<D> {
    /// Construct with given `direction`
    pub fn new_with_direction(direction: D) -> Self {
        ProgressBar { direction }
    }
}
impl<D: Directional, Data: SharedData<Item = f32>> Driver<f32, Data> for ProgressBar<D> {
    type Widget = kas_widgets::ProgressBar<D>;
    fn make(&self) -> Self::Widget {
        kas_widgets::ProgressBar::new_with_direction(self.direction)
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: f32) -> TkAction {
        widget.set_value(item)
    }
}

/// [`kas_widgets::CheckButton`] view widget constructor
#[derive(Clone, Debug, Default)]
pub struct CheckButton {
    label: AccelString,
}
impl CheckButton {
    /// Construct, with given `label`
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        let label = label.into();
        CheckButton { label }
    }
}
impl<Data: SharedData<Item = bool>> Driver<bool, Data> for CheckButton {
    type Widget = kas_widgets::CheckButton;
    fn make(&self) -> Self::Widget {
        kas_widgets::CheckButton::new_on(self.label.clone(), |mgr, state| mgr.push_msg(state))
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: bool) -> TkAction {
        widget.set_bool(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(state) = mgr.try_pop_msg() {
            data.update(mgr, key, state);
        }
    }
}

/// [`kas_widgets::RadioBox`] view widget constructor
#[derive(Clone, Debug)]
pub struct RadioBox {
    group: RadioGroup,
}
impl RadioBox {
    /// Construct, with given `group`
    pub fn new(group: RadioGroup) -> Self {
        RadioBox { group }
    }
}
impl<Data: SharedData<Item = bool>> Driver<bool, Data> for RadioBox {
    type Widget = kas_widgets::RadioBox;
    fn make(&self) -> Self::Widget {
        kas_widgets::RadioBox::new_on(self.group.clone(), |mgr| mgr.push_msg(true))
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: bool) -> TkAction {
        widget.set_bool(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(state) = mgr.try_pop_msg() {
            data.update(mgr, key, state);
        }
    }
}

/// [`kas_widgets::RadioButton`] view widget constructor
#[derive(Clone, Debug)]
pub struct RadioButton {
    label: AccelString,
    group: RadioGroup,
}
impl RadioButton {
    /// Construct, with given `label` and `group`
    pub fn new<T: Into<AccelString>>(label: T, group: RadioGroup) -> Self {
        let label = label.into();
        RadioButton { label, group }
    }
}
impl<Data: SharedData<Item = bool>> Driver<bool, Data> for RadioButton {
    type Widget = kas_widgets::RadioButton;
    fn make(&self) -> Self::Widget {
        kas_widgets::RadioButton::new(self.label.clone(), self.group.clone())
            .on_select(|mgr| mgr.push_msg(true))
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: bool) -> TkAction {
        widget.set_bool(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(state) = mgr.try_pop_msg() {
            data.update(mgr, key, state);
        }
    }
}

/// [`kas_widgets::Slider`] view widget constructor
#[derive(Clone, Copy, Debug)]
pub struct Slider<T: SliderValue, D: Directional> {
    range: (T, T),
    step: T,
    direction: D,
}
impl<T: SliderValue, D: Directional + Default> Slider<T, D> {
    /// Construct, with given `range` and `step` (see [`kas_widgets::Slider::new`])
    pub fn new(range: RangeInclusive<T>, step: T) -> Self {
        Slider {
            range: range.into_inner(),
            step,
            direction: D::default(),
        }
    }
}
impl<T: SliderValue, D: Directional> Slider<T, D> {
    /// Construct, with given `range`, `step` and `direction` (see [`Slider::new_with_direction`])
    pub fn new_with_direction(range: RangeInclusive<T>, step: T, direction: D) -> Self {
        Slider {
            range: range.into_inner(),
            step,
            direction,
        }
    }
}
impl<D: Directional, Data: SharedData> Driver<Data::Item, Data> for Slider<Data::Item, D>
where
    Data::Item: SliderValue,
{
    type Widget = kas_widgets::Slider<Data::Item, D>;
    fn make(&self) -> Self::Widget {
        let range = self.range.0..=self.range.1;
        kas_widgets::Slider::new_with_direction(range, self.step, self.direction)
            .on_move(|mgr, value| mgr.push_msg(value))
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: Data::Item) -> TkAction {
        widget.set_value(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(state) = mgr.try_pop_msg() {
            data.update(mgr, key, state);
        }
    }
}

/// [`kas_widgets::Spinner`] view widget constructor
#[derive(Clone, Copy, Debug)]
pub struct Spinner<T: SpinnerValue> {
    range: (T, T),
    step: T,
}
impl<T: SpinnerValue + Default> Spinner<T> {
    /// Construct, with given `range` and `step` (see [`kas_widgets::Spinner::new`])
    pub fn new(range: RangeInclusive<T>, step: T) -> Self {
        Spinner {
            range: range.into_inner(),
            step,
        }
    }
}
impl<Data: SharedData> Driver<Data::Item, Data> for Spinner<Data::Item>
where
    Data::Item: SpinnerValue,
{
    type Widget = kas_widgets::Spinner<Data::Item>;
    fn make(&self) -> Self::Widget {
        let range = self.range.0..=self.range.1;
        kas_widgets::Spinner::new(range, self.step).on_change(|mgr, val| mgr.push_msg(val))
    }
    fn set(&self, widget: &mut Self::Widget, _: &Data::Key, item: Data::Item) -> TkAction {
        widget.set_value(item)
    }
    fn on_message(&self, mgr: &mut EventMgr, _: &mut Self::Widget, data: &Data, key: &Data::Key) {
        if let Some(state) = mgr.try_pop_msg() {
            data.update(mgr, key, state);
        }
    }
}
