// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditGuard`] trait and some implementations

use super::Editor;
use kas::prelude::*;
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Event-handling *guard* for an [`Editor`]
///
/// This is the most generic interface; see also [`EditBox`] constructors for
/// common use-cases.
///
/// All methods have a default implementation which does nothing.
///
/// [`EditBox`]: super::EditBox
pub trait EditGuard: Sized {
    /// Data type
    type Data;

    /// Update guard
    ///
    /// This function is called when input data is updated **and** the editor
    /// does not have [input focus](Editor::has_input_focus).
    ///
    /// This method may also be called on loss of input focus (see
    /// [`Self::focus_lost`]).
    ///
    /// If this method is not required, consider implementing [`AutoEditGuard`]
    /// instead.
    fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &Self::Data);

    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes. Its result is returned
    /// from `handle_event`.
    ///
    /// The default implementation calls [`Self::focus_lost`] and returns [`Used`].
    #[inline]
    fn activate(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &Self::Data) -> IsUsed {
        self.focus_lost(edit, cx, data);
        Used
    }

    /// Focus-lost guard
    ///
    /// This function is called after the widget has lost both keyboard and IME
    /// focus.
    ///
    /// The default implementation calls [`Self::update`] since updates are
    /// inhibited while the editor has input focus.
    #[inline]
    fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &Self::Data) {
        self.update(edit, cx, data);
    }

    /// Edit guard
    ///
    /// This function is called after the text is updated (including by keyboard
    /// input, an undo action or by a message like
    /// [`kas::messages::SetValueText`]).
    ///
    /// The guard may call [`Editor::set_error`] here.
    /// The error state is cleared immediately before calling this method.
    #[inline]
    fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }
}

/// Event-handling *guard* for an independent [`Editor`]
///
/// This trait is for autonomous [`EditBox`]es: those containing their own data
/// instead of dependant on update from input data.
///
/// All methods have a default implementation which does nothing.
///
/// [`EditBox`]: super::EditBox
pub trait AutoEditGuard: Sized {
    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes. Its result is returned
    /// from `handle_event`.
    ///
    /// The default implementation calls [`Self::focus_lost`] and returns [`Used`].
    #[inline]
    fn activate(&mut self, edit: &mut Editor, cx: &mut EventCx) -> IsUsed {
        self.focus_lost(edit, cx);
        Used
    }

    /// Focus-lost guard
    ///
    /// This function is called after the widget has lost both keyboard and IME
    /// focus.
    ///
    /// The default implementation does nothing.
    #[inline]
    fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx) {
        let _ = (edit, cx);
    }

    /// Edit guard
    ///
    /// This function is called after the text is updated (including by keyboard
    /// input, an undo action or by a message like
    /// [`kas::messages::SetValueText`]).
    ///
    /// The guard may call [`Editor::set_error`] here.
    /// The error state is cleared immediately before calling this method.
    #[inline]
    fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx) {
        let _ = (edit, cx);
    }
}

impl<G: AutoEditGuard> EditGuard for G {
    type Data = ();

    #[inline]
    fn update(&mut self, _: &mut Editor, _: &mut ConfigCx, _: &Self::Data) {}

    #[inline]
    fn activate(&mut self, edit: &mut Editor, cx: &mut EventCx, _: &Self::Data) -> IsUsed {
        self.activate(edit, cx)
    }

    #[inline]
    fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, _: &Self::Data) {
        self.focus_lost(edit, cx);
    }

    #[inline]
    fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx, _: &Self::Data) {
        self.edit(edit, cx);
    }
}

/// Ignore all events and data updates
///
/// This guard should probably not be used for a functional user-interface but
/// may be useful in mock UIs.
#[autoimpl(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DefaultGuard;
impl AutoEditGuard for DefaultGuard {}

#[impl_self]
mod ReadGuard {
    /// An [`EditGuard`] for read-only content
    ///
    /// This may be used with read-only edit fields, essentially resulting in a
    /// fancier version of [`Text`](crate::Text) or
    /// [`ScrollText`](crate::ScrollText).
    #[autoimpl(Debug ignore self.value_fn)]
    pub struct ReadGuard<A> {
        value_fn: Box<dyn Fn(&A) -> String + Send>,
    }

    impl Self {
        /// Construct with a value function
        ///
        /// On update, `value_fn` is used to extract a value from input data.
        /// If, however, the input field has focus, the update is ignored.
        pub fn new(value_fn: impl Fn(&A) -> String + Send + 'static) -> Self {
            ReadGuard {
                value_fn: Box::new(value_fn),
            }
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &A) {
            let string = (self.value_fn)(data);
            edit.set_string(cx, string);
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
    #[autoimpl(Debug ignore self.on_afl)]
    pub struct ParseGuard<T: Debug + Display + FromStr<Err: Display>> {
        parsed: Option<T>,
        on_afl: Box<dyn Fn(&mut EventCx, T) + Send>,
    }

    impl Self {
        /// Construct
        ///
        /// On update, input data is formatted as a string via [`Display`].
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// On every edit, the guard attempts to parse the field's input as type
        /// `T` via [`FromStr`], caching the result and setting the error state.
        ///
        /// On field activation and focus loss when a `T` value is cached (see
        /// previous paragraph), `on_afl` is used to construct a message to be
        /// emitted via [`EventCx::push`]. The cached value is then cleared to
        /// avoid sending duplicate messages.
        pub fn new<M: Debug + 'static>(on_afl: impl Fn(T) -> M + Send + 'static) -> Self {
            ParseGuard {
                parsed: None,
                on_afl: Box::new(move |cx, value| cx.push(on_afl(value))),
            }
        }
    }

    impl EditGuard for Self {
        type Data = T;

        fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &T) {
            edit.set_string(cx, format!("{data}"));
            self.parsed = None;
        }

        fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &T) {
            if let Some(value) = self.parsed.take() {
                (self.on_afl)(cx, value);
            } else {
                // Reset data on focus loss (update is inhibited with focus).
                self.update(edit, cx, data);
            }
        }

        fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx, _: &T) {
            match edit.as_str().parse() {
                Ok(result) => self.parsed = Some(result),
                Err(err) => {
                    edit.set_error(cx, Some(format!("parse failure: {err}").into()));
                    self.parsed = None;
                }
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
    #[autoimpl(Debug ignore self.on_edit)]
    pub struct InstantParseGuard<T: Debug + Display + FromStr<Err: Display>> {
        on_edit: Box<dyn Fn(&mut EventCx, T) + Send>,
    }

    impl Self {
        /// Construct
        ///
        /// On update, input data is formatted as a string via [`Display`].
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// On every edit, the guard attempts to parse the field's input as type
        /// `T` via [`FromStr`]. On success, the result is converted to a
        /// message via `on_edit` then emitted via [`EventCx::push`].
        pub fn new<M: Debug + 'static>(on_edit: impl Fn(T) -> M + Send + 'static) -> Self {
            InstantParseGuard {
                on_edit: Box::new(move |cx, value| cx.push(on_edit(value))),
            }
        }
    }

    impl EditGuard for Self {
        type Data = T;

        fn update(&mut self, edit: &mut Editor, cx: &mut ConfigCx, data: &T) {
            edit.set_string(cx, format!("{data}"));
        }

        fn focus_lost(&mut self, edit: &mut Editor, cx: &mut EventCx, data: &T) {
            // Always reset data on focus loss
            self.update(edit, cx, data);
        }

        fn edit(&mut self, edit: &mut Editor, cx: &mut EventCx, _: &T) {
            match edit.as_str().parse() {
                Ok(result) => (self.on_edit)(cx, result),
                Err(err) => {
                    edit.set_error(cx, Some(format!("parse failure: {err}").into()));
                }
            }
        }
    }
}
