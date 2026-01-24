// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditGuard`] trait and some implementations

use super::Editor;
use kas::prelude::*;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::str::FromStr;

/// Event-handling *guard* for an [`Editor`]
///
/// This is the most generic interface; see also constructors of [`EditField`],
/// [`EditBox`] for common use-cases.
///
/// All methods have a default implementation which does nothing.
///
/// [`EditBox`]: super::EditBox
/// [`EditField`]: super::EditField
pub trait EditGuard: Sized {
    /// Data type
    type Data;

    /// Configure guard
    ///
    /// This function is called when the attached widget is configured.
    fn configure(&mut self, edit: &mut Editor, cx: &mut ConfigCx) {
        let _ = (edit, cx);
    }

    /// Update guard
    ///
    /// This function is called when input data is updated.
    ///
    /// Note that this method may be called during editing as a result of a
    /// message sent by the [`Editor`] or another cause. It is recommended to
    /// ignore updates for editable widgets with
    /// [key focus](Editor::has_edit_focus) to avoid overwriting user input;
    /// [`Self::focus_lost`] may update the content instead.
    /// For read-only fields this is not recommended (but `has_edit_focus` will
    /// not be true anyway).
    fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }

    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes. Its result is returned
    /// from `handle_event`.
    ///
    /// The default implementation:
    ///
    /// -   If the field is editable, calls [`Self::focus_lost`] and returns
    ///     returns [`Used`].
    /// -   If the field is not editable, returns [`Unused`].
    fn activate(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &Self::Data) -> IsUsed {
        if edit.is_editable() {
            self.focus_lost(edit, cx, data);
            Used
        } else {
            Unused
        }
    }

    /// Focus-gained guard
    ///
    /// This function is called when the widget gains keyboard or IME focus.
    fn focus_gained(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }

    /// Focus-lost guard
    ///
    /// This function is called after the widget has lost both keyboard and IME
    /// focus.
    fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }

    /// Edit guard
    ///
    /// This function is called after the text is updated (including by keyboard
    /// input, an undo action or by a message like
    /// [`kas::messages::SetValueText`]). The exceptions are setter methods like
    /// [`clear`](Editor::clear) and [`set_string`](Editor::set_string).
    ///
    /// The guard may set the [error state](Editor::set_error_state) here.
    /// The error state is cleared immediately before calling this method.
    fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }
}

/// Ignore all events and data updates
///
/// This guard should probably not be used for a functional user-interface but
/// may be useful in mock UIs.
#[autoimpl(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DefaultGuard<A>(PhantomData<A>);
impl<A: 'static> EditGuard for DefaultGuard<A> {
    type Data = A;
}

#[impl_self]
mod StringGuard {
    /// An [`EditGuard`] for read-only strings
    ///
    /// This may be used with read-only edit fields, essentially resulting in a
    /// fancier version of [`Text`](crate::Text) or
    /// [`ScrollText`](crate::ScrollText).
    #[autoimpl(Debug ignore self.value_fn, self.on_afl)]
    pub struct StringGuard<A> {
        value_fn: Box<dyn Fn(&A) -> String>,
        on_afl: Option<Box<dyn Fn(&mut EventCx, &A, &str)>>,
        edited: bool,
    }

    impl Self {
        /// Construct with a value function
        ///
        /// On update, `value_fn` is used to extract a value from input data.
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// No other action happens unless [`Self::with_msg`] is used.
        pub fn new(value_fn: impl Fn(&A) -> String + 'static) -> Self {
            StringGuard {
                value_fn: Box::new(value_fn),
                on_afl: None,
                edited: false,
            }
        }

        /// Call the handler `f` on activation / focus loss
        ///
        /// On field **a**ctivation and **f**ocus **l**oss (AFL) after an edit,
        /// `f` is called.
        pub fn with(mut self, f: impl Fn(&mut EventCx, &A, &str) + 'static) -> Self {
            debug_assert!(self.on_afl.is_none());
            self.on_afl = Some(Box::new(f));
            self
        }

        /// Send the message generated by `f` on activation / focus loss
        ///
        /// On field **a**ctivation and **f**ocus **l**oss (AFL) after an edit,
        /// `f` is used to construct a message to be emitted via [`EventCx::push`].
        pub fn with_msg<M: Debug + 'static>(self, f: impl Fn(&str) -> M + 'static) -> Self {
            self.with(move |cx, _, value| cx.push(f(value)))
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &A) {
            if self.edited {
                self.edited = false;
                if let Some(ref on_afl) = self.on_afl {
                    return on_afl(cx, data, edit.as_str());
                }
            }

            // Reset data on focus loss (update is inhibited with focus).
            // No need if we just sent a message (should cause an update).
            let string = (self.value_fn)(data);
            edit.set_string(cx, string);
        }

        fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &A) {
            if !edit.has_edit_focus() {
                let string = (self.value_fn)(data);
                edit.set_string(cx, string);
            }
        }

        fn edit(&mut self, _: &mut Editor, _: &mut EventCx, _: &Self::Data) {
            self.edited = true;
        }
    }
}

#[impl_self]
mod ParseGuard {
    /// An [`EditGuard`] for parsable types
    ///
    /// This guard displays a value formatted from input data, updates the error
    /// state according to parse success on each keystroke, and sends a message
    /// on focus loss (where successful parsing occurred).
    #[autoimpl(Debug ignore self.value_fn, self.on_afl)]
    pub struct ParseGuard<A, T: Debug + Display + FromStr> {
        parsed: Option<T>,
        value_fn: Box<dyn Fn(&A) -> T>,
        on_afl: Box<dyn Fn(&mut EventCx, T)>,
    }

    impl Self {
        /// Construct
        ///
        /// On update, `value_fn` is used to extract a value from input data
        /// which is then formatted as a string via [`Display`].
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// On every edit, the guard attempts to parse the field's input as type
        /// `T` via [`FromStr`], caching the result and setting the error state.
        ///
        /// On field activation and focus loss when a `T` value is cached (see
        /// previous paragraph), `on_afl` is used to construct a message to be
        /// emitted via [`EventCx::push`]. The cached value is then cleared to
        /// avoid sending duplicate messages.
        pub fn new<M: Debug + 'static>(
            value_fn: impl Fn(&A) -> T + 'static,
            on_afl: impl Fn(T) -> M + 'static,
        ) -> Self {
            ParseGuard {
                parsed: None,
                value_fn: Box::new(value_fn),
                on_afl: Box::new(move |cx, value| cx.push(on_afl(value))),
            }
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &A) {
            if let Some(value) = self.parsed.take() {
                (self.on_afl)(cx, value);
            } else {
                // Reset data on focus loss (update is inhibited with focus).
                // No need if we just sent a message (should cause an update).
                let value = (self.value_fn)(data);
                edit.set_string(cx, format!("{value}"));
            }
        }

        fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx, _: &A) {
            self.parsed = edit.as_str().parse().ok();
            let is_err = self.parsed.is_none();
            edit.set_error_state(cx, is_err);
        }

        fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &A) {
            if !edit.has_edit_focus() {
                let value = (self.value_fn)(data);
                edit.set_string(cx, format!("{value}"));
                self.parsed = None;
            }
        }
    }
}

#[impl_self]
mod InstantParseGuard {
    /// An as-you-type [`EditGuard`] for parsable types
    ///
    /// This guard displays a value formatted from input data, updates the error
    /// state according to parse success on each keystroke, and sends a message
    /// immediately (where successful parsing occurred).
    #[autoimpl(Debug ignore self.value_fn, self.on_afl)]
    pub struct InstantParseGuard<A, T: Debug + Display + FromStr> {
        value_fn: Box<dyn Fn(&A) -> T>,
        on_afl: Box<dyn Fn(&mut EventCx, T)>,
    }

    impl Self {
        /// Construct
        ///
        /// On update, `value_fn` is used to extract a value from input data
        /// which is then formatted as a string via [`Display`].
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// On every edit, the guard attempts to parse the field's input as type
        /// `T` via [`FromStr`]. On success, the result is converted to a
        /// message via `on_afl` then emitted via [`EventCx::push`].
        pub fn new<M: Debug + 'static>(
            value_fn: impl Fn(&A) -> T + 'static,
            on_afl: impl Fn(T) -> M + 'static,
        ) -> Self {
            InstantParseGuard {
                value_fn: Box::new(value_fn),
                on_afl: Box::new(move |cx, value| cx.push(on_afl(value))),
            }
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &A) {
            // Always reset data on focus loss
            let value = (self.value_fn)(data);
            edit.set_string(cx, format!("{value}"));
        }

        fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx, _: &A) {
            let result = edit.as_str().parse();
            edit.set_error_state(cx, result.is_err());
            if let Ok(value) = result {
                (self.on_afl)(cx, value);
            }
        }

        fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &A) {
            if !edit.has_edit_focus() {
                let value = (self.value_fn)(data);
                edit.set_string(cx, format!("{value}"));
            }
        }
    }
}
