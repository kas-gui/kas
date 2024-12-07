// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::theme::{self, TextClass};

impl_scope! {
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
    pub struct Text<A, T: Default + FormattableText + 'static> {
        core: widget_core!(),
        text: theme::Text<T>,
        text_fn: Box<dyn Fn(&ConfigCx, &A) -> T>,
    }

    impl Default for Self where for<'a> &'a A: Into<T> {
        fn default() -> Self {
            Text {
                core: Default::default(),
                text: theme::Text::new(T::default(), TextClass::Label(true)),
                text_fn: Box::new(|_, data| data.into()),
            }
        }
    }

    impl Self {
        /// Construct with a data binding
        #[inline]
        pub fn new(text_fn: impl Fn(&ConfigCx, &A) -> T + 'static) -> Self {
            Text {
                core: Default::default(),
                text: theme::Text::new(T::default(), TextClass::Label(true)),
                text_fn: Box::new(text_fn),
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
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            sizer.text_rules(&mut self.text, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.core.rect = rect;
            let align = hints.complete(Align::Default, Align::Center);
            cx.text_set_size(&mut self.text, rect.size, align);
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.text(self.rect(), &self.text);
        }
    }

    impl Events for Self {
        type Data = A;

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            let text = (self.text_fn)(cx, data);
            if text.as_str() == self.text.as_str() {
                // NOTE(opt): avoiding re-preparation of text is a *huge*
                // optimisation. Move into kas-text?
                return;
            }
            self.text.set_text(text);
            let action = self.text.reprepare_action();
            cx.action(self, action);
        }
    }
}

/* TODO(specialization): can we support this? min_specialization is not enough.
impl<U, T: From<U> + FormattableText + 'static> From<U> for Text<T> {
    default fn from(text: U) -> Self {
        let text = T::from(text);
        Text::new(text)
    }
}*/

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
        $crate::Text::new(move |_, $data| format!($($arg)*))
    };
    ($data:ident : $data_ty:ty , $($arg:tt)*) => {
        $crate::Text::new(move |_, $data : $data_ty| format!($($arg)*))
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
        $crate::Text::new(move |_, data| format!($($arg)*, data))
    };
}
