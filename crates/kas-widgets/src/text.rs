// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::prelude::*;
use kas::text::{self, format::FormattableText};
use kas::theme::TextClass;

// NOTE: Text maintains a copy of the (formatted) string internally. Most
// importantly this allows change detection.

#[impl_self]
mod Text {
    /// A text label (derived from data)
    ///
    /// `Text` derives its contents from input data. Use [`Label`](crate::Label)
    /// instead for fixed contents.
    ///
    /// By default, this uses [`TextClass::Standard`]; see [`Self::set_class`]
    /// and [`Self::with_class`].
    ///
    /// See also macros [`format_text`](super::format_text) and
    /// [`format_label`](super::format_label) which construct a
    /// `Text` widget.
    ///
    /// Vertical alignment defaults to centred, horizontal alignment depends on
    /// the script direction if not specified. Line-wrapping is enabled by
    /// default.
    #[widget]
    #[layout(self.text)]
    pub struct Text<A, T: Default + FormattableText + 'static = String> {
        core: widget_core!(),
        text: text::Text<T>,
        text_fn: Box<dyn Fn(&A, &mut T) -> bool + Send>,
    }

    impl Default for Self
    where
        for<'a> &'a A: Into<T>,
    {
        fn default() -> Self {
            Text {
                core: Default::default(),
                text: text::Text::new(T::default(), TextClass::Standard, true),
                text_fn: Box::new(|data, text| {
                    let new_text = data.into();
                    let changed = new_text != *text;
                    if changed {
                        *text = new_text;
                    }
                    changed
                }),
            }
        }
    }

    impl<A> Text<A, String> {
        /// Construct with an `str` accessor
        pub fn new_str(as_str: impl Fn(&A) -> &str + Send + 'static) -> Self {
            Text {
                core: Default::default(),
                text: text::Text::new(String::new(), TextClass::Standard, true),
                text_fn: Box::new(move |data, text| {
                    let s = as_str(data);
                    let changed = *text != *s;
                    if changed {
                        *text = s.into();
                    }
                    changed
                }),
            }
        }
    }

    impl Self {
        /// Construct with a generator function
        ///
        /// `gen_text` is called on each widget update to generate text from
        /// input data.
        pub fn new_gen(gen_text: impl Fn(&A) -> T + Send + 'static) -> Self {
            Text {
                core: Default::default(),
                text: text::Text::new(T::default(), TextClass::Standard, true),
                text_fn: Box::new(move |data, text| {
                    let new_text = gen_text(data);
                    let changed = new_text != *text;
                    if changed {
                        *text = new_text;
                    }
                    changed
                }),
            }
        }

        /// Construct with an update function
        ///
        /// `update_text` is called on each widget update to generate text from
        /// input data. It must return `true` when the input text is changed (or
        /// updated text will not be displayed) and should return `false`
        /// otherwise (or the text will be re-prepared needlessly, which can be
        /// expensive).
        pub fn new_update<U>(update_text: U) -> Self
        where
            U: Fn(&A, &mut T) -> bool + Send + 'static,
        {
            Text {
                core: Default::default(),
                text: text::Text::new(T::default(), TextClass::Standard, true),
                text_fn: Box::new(update_text),
            }
        }

        /// Get text class
        #[inline]
        pub fn class(&self) -> TextClass {
            self.text.class()
        }

        /// Set text class
        ///
        /// Default: [`TextClass::Standard`]
        #[inline]
        pub fn set_class(&mut self, class: TextClass) {
            self.text.set_class(class);
        }

        /// Set text class (inline)
        ///
        /// Default: [`TextClass::Standard`]
        #[inline]
        pub fn with_class(mut self, class: TextClass) -> Self {
            self.text.set_class(class);
            self
        }

        /// Get whether line-wrapping is enabled
        #[inline]
        pub fn wrap(&self) -> bool {
            self.text.wrap()
        }

        /// Enable/disable line wrapping
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.text.set_wrap(wrap);
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.text.set_wrap(wrap);
            self
        }

        /// Get read access to the text object
        #[inline]
        pub fn text(&self) -> &text::Text<T> {
            &self.text
        }

        /// Read the text contents as an `str`
        #[inline]
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            self.text
                .set_rect(cx, rect, hints.combine(AlignHints::VERT_CENTER));
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Label(self.text.as_str())
        }
    }

    impl Events for Self {
        type Data = A;

        fn configure(&mut self, cx: &mut ConfigCx) {
            self.text.configure(&mut cx.size_cx());
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            if (self.text_fn)(data, self.text.text_mut()) {
                self.text.require_reprepare();
                self.text.reprepare_action(cx);
            }
        }
    }
}

/// Construct a [`Text`] widget using [`TextClass::Standard`] which updates text
/// using the [`format!`] macro
///
/// This macro is syntactic sugar over [`Text::new_gen`], using [`format!`] to
/// generate [`String`] content each time the widget is updated.
///
/// This uses [`TextClass::Standard`]. See also [`format_label`](crate::format_label)
/// which is identical aside from the [`TextClass`].
///
/// Examples:
/// ```
/// use kas_widgets::Text;
/// let _ = kas_widgets::format_text!(data: &i32, "Data value: {data}");
/// let _: Text<i32, _> = kas_widgets::format_text!(data, "Data value: {data}");
/// let _: Text<i32, String> = kas_widgets::format_text!("Data value: {}");
/// ```
#[macro_export]
macro_rules! format_text {
    ($data:ident, $($arg:tt)*) => {
        $crate::Text::new_gen(move |$data| format!($($arg)*))
    };
    ($data:ident : $data_ty:ty , $($arg:tt)*) => {
        $crate::Text::new_gen(move |$data : $data_ty| format!($($arg)*))
    };
    ($lit:literal $(, $arg:tt)*) => {
        $crate::Text::new_gen(move |data| format!($lit $(, $arg)*, data))
    };
}

/// Construct a [`Text`] widget using [`TextClass::Label`] which updates text
/// using the [`format!`] macro
///
/// This macro is syntactic sugar over [`Text::new_gen`], using [`format!`] to
/// generate [`String`] content each time the widget is updated.
///
/// This uses [`TextClass::Label`]. See also [`format_text`](crate::format_text)
/// which is identical aside from the [`TextClass`].
#[macro_export]
macro_rules! format_label {
    ($data:ident, $($arg:tt)*) => {
        $crate::Text::new_gen(move |$data| format!($($arg)*))
            .with_class(::kas::theme::TextClass::Label)
    };
    ($data:ident : $data_ty:ty , $($arg:tt)*) => {
        $crate::Text::new_gen(move |$data : $data_ty| format!($($arg)*))
            .with_class(::kas::theme::TextClass::Label)
    };
    ($lit:literal $(, $arg:tt)*) => {
        $crate::Text::new_gen(move |data| format!($lit $(, $arg)*, data))
            .with_class(::kas::theme::TextClass::Label)
    };
}
