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
//! -   [`EditBox`], [`CheckBox`] etc. provide an interactive view over common
//!     data types using the like-named widgets
//! -   [`EventConfig`] provides an editor over a specific complex data type
//!
//! Intended usage is to import the module name rather than its contents, thus
//! allowing referal to e.g. `driver::View`.

// mod config;
// pub use config::EventConfig;

use crate::MaybeOwned;
use kas::model::SharedData;
use kas::prelude::*;
use kas_widgets::{Label, NavFrame};
use std::default::Default;

/// View widget driver/binder
///
/// The driver can:
///
/// -   construct (empty) widgets with [`Self::make`]
/// -   assign data to an existing widget with [`Self::set`]
/// -   (optional) handle messages from a widget with [`Self::on_messages`]
///
/// NOTE: `Item` is a direct type parameter (in addition to an assoc. type
/// param. of `SharedData`) only to avoid "conflicting implementations" errors.
/// Similar to: rust#20400, rust#92894. Given fixes, we may remove the param.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, std::rc::Rc<T>, std::sync::Arc<T>)]
pub trait Driver<Item, Data: SharedData<Item = Item>> {
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
    ///
    /// This method is a convenience wrapper around [`Self::set_mo`].
    fn set<'b>(
        &self,
        widget: &mut Self::Widget,
        key: &Data::Key,
        item: impl Into<MaybeOwned<'b, Item>>,
    ) -> Action
    where
        Self: Sized,
        Item: 'b,
    {
        self.set_mo(widget, key, item.into())
    }

    /// Set the viewed data ([`MaybeOwned`])
    ///
    /// The widget may expect `configure` to be called at least once before data
    /// is set and to have `set_rect` called after each time data is set.
    fn set_mo(&self, widget: &mut Self::Widget, key: &Data::Key, item: MaybeOwned<Item>) -> Action;

    /// Handle a message from a widget
    ///
    /// This method is called when a view widget returns with a message; it
    /// may retrieve this message with [`EventMgr::try_pop`].
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
    ///     message and updates `data` using values read from `widget`.
    ///
    /// See, for example, the implementation for [`CheckButton`]: the `make`
    /// method assigns a state-change handler which `on_messages` uses to update
    /// the shared data.
    ///
    /// Default implementation: do nothing.
    fn on_messages(
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
            fn set_mo(&self, widget: &mut Self::Widget, _: &Data::Key, item: MaybeOwned<$t>) -> Action {
                widget.set_string(item.to_string())
            }
        }
        impl<Data: SharedData<Item = $t>> Driver<$t, Data> for NavView {
            type Widget = NavFrame<Label<String>>;
            fn make(&self) -> Self::Widget {
                NavFrame::new(Label::new("".to_string()))
            }
            fn set_mo(&self, widget: &mut Self::Widget, _: &Data::Key, item: MaybeOwned<$t>) -> Action {
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
