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
//! A basic implementation is provided: [`View`] provides a simple read-only
//! view over content (text labels in all cases except `bool`, which uses a
//! read-only [`CheckBox`]).
//!
//! Intended usage is to import the module name rather than its contents, thus
//! allowing referal to e.g. `driver::View`.

use kas::TextOrSource;
use kas::prelude::*;
use kas_widgets::{CheckBox, Text};
use std::default::Default;

/// View widget driver
///
/// Implementations of this trait [make](Self::make) new view widgets and
/// optionally [reassign](Self::set_key) existing view widgets.
///
/// View widgets may also need to update themselves using [`Events::update`].
///
/// Each view widget has an [`Id`] corresponding to its data item, and
/// handles events like any other widget. In order to associate a returned
/// message with a `Key`, either embed that key while constructing
/// the widget with [`Driver::make`] or handle the message in
/// [`crate::DataClerk::handle_messages`].
///
/// # Example implementations
///
/// It is expected that a custom implementation is created for each usage. A
/// simple example might just map input data to a [`Text`] widget:
/// ```
/// use kas_view::Driver;
/// use kas_widgets::Text;
///
/// struct MyDriver;
/// impl<Key> Driver<Key, f32> for MyDriver {
///     type Widget = Text<f32, String>;
///     fn make(&mut self, _: &Key) -> Self::Widget {
///         Text::new(|_, data: &f32| data.to_string())
///     }
///     fn set_key(&mut self, _: &mut Self::Widget, _: &Key) {
///         // Text has no metadata that needs to be reset
///     }
/// }
/// ```
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

    /// Whether the `Widget` wrapper should be keyboard navigable
    fn navigable(widget: &Self::Widget) -> bool;

    /// Get optional label for widgets
    ///
    /// This allows accessibility tools to read an item's label on focus. For complex
    /// widgets supporting focus this may not be wanted. Defaults to `None`.
    fn label(widget: &Self::Widget) -> Option<TextOrSource<'_>> {
        let _ = widget;
        None
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
            fn navigable(_: &Self::Widget) -> bool {
                true
            }
            fn label(widget: &Self::Widget) -> Option<TextOrSource<'_>> {
                Some(widget.id().into())
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
    fn navigable(_: &Self::Widget) -> bool {
        false
    }
}
