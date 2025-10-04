// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::theme::{self, TextClass};

#[impl_self]
mod Text {
    /// A text label (derived from data)
    ///
    /// `Text` derives its contents from input data. Use [`Label`](crate::Label)
    /// instead for fixed contents.
    ///
    /// See also macros [`format_data`](super::format_data) and
    /// [`format_value`](super::format_value) which construct a
    /// `Text` widget.
    ///
    /// Vertical alignment defaults to centred, horizontal alignment depends on
    /// the script direction if not specified. Line-wrapping is enabled by
    /// default.
    #[widget]
    #[layout(self.text)]
    pub struct Text<A, T: Default + FormattableText + 'static> {
        core: widget_core!(),
        text: theme::Text<T>,
        text_fn: Box<dyn Fn(&ConfigCx, &A, &mut T) -> bool>,
    }

    impl Default for Self
    where
        for<'a> &'a A: Into<T>,
    {
        fn default() -> Self {
            Text {
                core: Default::default(),
                text: theme::Text::new(T::default(), TextClass::Label(true)),
                text_fn: Box::new(|_, data, text| {
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
        pub fn new_str(as_str: impl Fn(&A) -> &str + 'static) -> Self {
            Text {
                core: Default::default(),
                text: theme::Text::new(String::new(), TextClass::Label(true)),
                text_fn: Box::new(move |_, data, text| {
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
        pub fn new_gen(gen_text: impl Fn(&ConfigCx, &A) -> T + 'static) -> Self {
            Text {
                core: Default::default(),
                text: theme::Text::new(T::default(), TextClass::Label(true)),
                text_fn: Box::new(move |cx, data, text| {
                    let new_text = gen_text(cx, data);
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
        pub fn new_update(update_text: impl Fn(&ConfigCx, &A, &mut T) -> bool + 'static) -> Self {
            Text {
                core: Default::default(),
                text: theme::Text::new(T::default(), TextClass::Label(true)),
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
        /// Default: `TextClass::Label(true)`
        #[inline]
        pub fn set_class(&mut self, class: TextClass) {
            self.text.set_class(class);
        }

        /// Set text class (inline)
        ///
        /// Default: `TextClass::Label(true)`
        #[inline]
        pub fn with_class(mut self, class: TextClass) -> Self {
            self.text.set_class(class);
            self
        }

        /// Get whether line-wrapping is enabled
        #[inline]
        pub fn wrap(&self) -> bool {
            self.class().multi_line()
        }

        /// Enable/disable line wrapping
        ///
        /// This is equivalent to `label.set_class(TextClass::Label(wrap))`.
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.text.set_class(TextClass::Label(wrap));
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.text.set_class(TextClass::Label(wrap));
            self
        }

        /// Get read access to the text object
        #[inline]
        pub fn text(&self) -> &theme::Text<T> {
            &self.text
        }

        /// Read the text contents as an `str`
        #[inline]
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
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
            cx.text_configure(&mut self.text);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            if (self.text_fn)(cx, data, self.text.text_mut()) {
                self.text.require_reprepare();
                let action = self.text.reprepare_action();
                cx.action(self, action);
            }
        }
    }
}

/// A [`Text`] widget which formats a value from input
///
/// Examples:
/// ```
/// use kas_widgets::Text;
/// let _: Text<i32, _> = kas_widgets::format_data!(data, "Data value: {data}");
/// let _ = kas_widgets::format_data!(data: &i32, "Data value: {data}");
/// ```
// TODO: a more fancy macro could determine the data fields used and wrap with
// a node testing for changes to these fields before calling update().
#[macro_export]
macro_rules! format_data {
    ($data:ident, $($arg:tt)*) => {
        $crate::Text::new_gen(move |_, $data| format!($($arg)*))
    };
    ($data:ident : $data_ty:ty , $($arg:tt)*) => {
        $crate::Text::new_gen(move |_, $data : $data_ty| format!($($arg)*))
    };
}

/// A [`Text`] widget which formats a value from input
///
/// Example:
/// ```
/// use kas_widgets::Text;
/// let _: Text<i32, String> = kas_widgets::format_value!("Data value: {}");
/// ```
#[macro_export]
macro_rules! format_value {
    ($($arg:tt)*) => {
        $crate::Text::new_gen(move |_, data| format!($($arg)*, data))
    };
}
