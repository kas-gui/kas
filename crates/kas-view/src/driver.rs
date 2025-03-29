// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View drivers
//!
//! The [`Driver`] trait is used as a binding between data models and
//! controllers. Implementations define the view (using widgets) and
//! message handling (mapping widget messages to actions).
//!
//! Several implementations are provided to cover simpler cases:
//!
//! -   [`View`] is the default, providing a simple read-only view over content
//! -   [`NavView`] is like [`View`], but using keyboard navigable widgets
//!
//! Intended usage is to import the module name rather than its contents, thus
//! allowing referal to e.g. `driver::View`.

use kas::prelude::*;
use kas_widgets::{CheckBox, NavFrame, Text};
use std::default::Default;

/// View widget driver
///
/// This trait is implemented to "drive" a controller widget,
/// constructing, updating, and optionally intercepting messages from so called
/// "view widgets".
/// A few simple implementations are provided: [`View`], [`NavView`].
///
/// Each view widget has an [`Id`] corresponding to its data item, and
/// handles events like any other widget. In order to associate a returned
/// message with a `Key`, either embed that key while constructing
/// the widget with [`Driver::make`] or intercept the message in
/// [`Driver::handle_messages`].
#[autoimpl(for<T: trait + ?Sized> &mut T, Box<T>)]
pub trait Driver<Key, Item> {
    /// Type of the widget used to view data
    type Widget: kas::Widget<Data = Item>;

    /// Construct a new view widget
    fn make(&mut self, key: &Key) -> Self::Widget;

    /// Called to bind an existing view widget to a new key
    ///
    /// This should reset widget metadata, for example so that when a view
    /// widget with a text selection is assigned to a new key it does not
    /// attempt to apply the old selection to the new text.
    ///
    /// This does not need to set data; [`Events::update`] does that.
    ///
    /// The default implementation simply replaces widget with `self.make(key)`,
    /// which is sufficient, if not always optimal.
    fn set_key(&mut self, widget: &mut Self::Widget, key: &Key) {
        *widget = self.make(key);
    }

    /// Handle a message from a widget
    ///
    /// This method is called when a view widget returns a message. Often
    /// it won't be used, but it may, for example, [pop](EventCx::try_pop) a
    /// message then [push](EventCx::push) a new one with the associated `key`.
    ///
    /// Default implementation: do nothing.
    fn handle_messages(
        &mut self,
        cx: &mut EventCx,
        widget: &mut Self::Widget,
        data: &Item,
        key: &Key,
    ) {
        let _ = (cx, data, key, widget);
    }
}

/// Default view widget constructor
///
/// This struct implements [`Driver`], using a default widget for the data type:
///
/// -   [`kas_widgets::Text`] for `String`, `&str`, integer and float types
/// -   [`kas_widgets::CheckBox`] (read-only) for the bool type TODO
#[derive(Clone, Copy, Debug, Default)]
pub struct View;

/// Default view widget constructor supporting keyboard navigation
///
/// This struct implements [`Driver`], using a default widget for the data type
/// which also supports keyboard navigation:
///
/// -   [`kas_widgets::NavFrame`] around a [`kas_widgets::Text`] for `String`, `&str`,
///     integer and float types
/// -   [`kas_widgets::CheckBox`] (read-only) for the bool type
#[derive(Clone, Copy, Debug, Default)]
pub struct NavView;

macro_rules! impl_via_to_string {
    ($t:ty) => {
        impl<Key> Driver<Key, $t> for View {
            type Widget = Text<$t, String>;
            fn make(&mut self, _: &Key) -> Self::Widget {
                Text::new(|_, data: &$t| data.to_string())
            }
            fn set_key(&mut self, _: &mut Self::Widget, _: &Key) {
                // Text has no metadata that needs to be reset
            }
        }
        impl<Key> Driver<Key, $t> for NavView {
            type Widget = NavFrame<Text<$t, String>>;
            fn make(&mut self, _: &Key) -> Self::Widget {
                NavFrame::new(Text::new(|_, data: &$t| data.to_string()))
            }
            fn set_key(&mut self, _: &mut Self::Widget, _: &Key) {
                // NavFrame and Text have no metadata that needs to be reset
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

impl<Key> Driver<Key, bool> for View {
    type Widget = CheckBox<bool>;
    fn make(&mut self, _: &Key) -> Self::Widget {
        CheckBox::new(|_, data: &bool| *data).with_editable(false)
    }
    fn set_key(&mut self, _: &mut Self::Widget, _: &Key) {
        // CheckBox has no metadata that needs to be reset
    }
}
impl<Key> Driver<Key, bool> for NavView {
    type Widget = CheckBox<bool>;
    fn make(&mut self, _: &Key) -> Self::Widget {
        CheckBox::new(|_, data: &bool| *data).with_editable(false)
    }
    fn set_key(&mut self, _: &mut Self::Widget, _: &Key) {
        // CheckBox has no metadata that needs to be reset
    }
}
