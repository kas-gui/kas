// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::prelude::*;
use kas::text;
use kas::text::format::FormattableText;
use kas::theme::TextClass;
use std::marker::PhantomData;

impl_scope! {
    /// A text label (derived from data)
    ///
    /// `Text` derives its contents from input data. Use [`Label`](crate::Label)
    /// instead for fixed contents.
    ///
    /// A text label. Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled by default.
    ///
    /// This type is generic over the text type.
    /// See also: [`StrText`], [`StringText`], [`AccelText`].
    #[autoimpl(Debug ignore self.label_fn, self._data)]
    #[widget {
        data = A;
    }]
    pub struct Text<A, T: Default + FormattableText + 'static> {
        core: widget_core!(),
        class: TextClass,
        label: text::Text<T>,
        label_fn: Box<dyn Fn(&A) -> T>,
        _data: PhantomData<A>,
    }

    impl Self {
        /// Construct with a data binding
        #[inline]
        pub fn new(label_fn: impl Fn(&A) -> T + 'static) -> Self {
            Text {
                core: Default::default(),
                class: TextClass::Label(true),
                label: text::Text::new(T::default()),
                label_fn: Box::new(label_fn),
                _data: PhantomData,
            }
        }

        /// Get text class
        #[inline]
        pub fn class(&self) -> TextClass {
            self.class
        }

        /// Set text class
        ///
        /// Default: `TextClass::Label(true)`
        #[inline]
        pub fn set_class(&mut self, class: TextClass) {
            self.class = class;
        }

        /// Set text class (inline)
        ///
        /// Default: `TextClass::Label(true)`
        #[inline]
        pub fn with_class(mut self, class: TextClass) -> Self {
            self.class = class;
            self
        }

        /// Get whether line-wrapping is enabled
        #[inline]
        pub fn wrap(&self) -> bool {
            self.class.multi_line()
        }

        /// Enable/disable line wrapping
        ///
        /// This is equivalent to `label.set_class(TextClass::Label(wrap))`.
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.class = TextClass::Label(wrap);
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.class = TextClass::Label(wrap);
            self
        }

        /// Get read access to the text object
        #[inline]
        pub fn text(&self) -> &text::Text<T> {
            &self.label
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Default, Align::Center);
            size_mgr.text_rules(&mut self.label, self.class, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;
            mgr.text_set_size(&mut self.label, self.class, rect.size, None);
        }

        #[cfg(feature = "min_spec")]
        default fn draw(&mut self, mut draw: DrawMgr) {
            draw.text_effects(self.rect(), &self.label, self.class);
        }
        #[cfg(not(feature = "min_spec"))]
        fn draw(&mut self, mut draw: DrawMgr) {
            draw.text_effects(self.rect(), &self.label, self.class);
        }
    }

    impl Widget for Self {
        fn update(&mut self, cx: &mut ConfigCx<A>) {
            let text = (self.label_fn)(cx.data());
            match self.label.set_and_try_prepare(text) {
                Ok(true) => *cx |= Action::RESIZE,
                _ => cx.redraw(self.id()),
            }
        }
    }
}

// Str/String representations have no effects, so use simpler draw call
#[cfg(feature = "min_spec")]
impl<'a, A> Layout for Text<A, &'a str> {
    fn draw(&mut self, mut draw: DrawMgr) {
        draw.text(self.rect(), &self.label, self.class);
    }
}
#[cfg(feature = "min_spec")]
impl<A> Layout for StringText<A> {
    fn draw(&mut self, mut draw: DrawMgr) {
        draw.text(self.rect(), &self.label, self.class);
    }
}

/* TODO(specialization): can we support this? min_specialization is not enough.
impl<U, T: From<U> + FormattableText + 'static> From<U> for Text<T> {
    default fn from(text: U) -> Self {
        let text = T::from(text);
        Text::new(text)
    }
}*/

/// Text with `&'static str` as backing type
pub type StrText<A> = Text<A, &'static str>;

/// Text with `String` as backing type
pub type StringText<A> = Text<A, String>;

// TODO: a more fancy macro could determine the data fields used and wrap with
// a node testing for changes to these fields before calling update().
#[macro_export]
macro_rules! format_text {
    ($data:ident, $($arg:tt)*) => {
        Text::new(move |$data, label: &mut String| {
            use std::fmt::Write;
            label.clear();
            write!(label, $($arg)*)
                .expect("format_text: formatting error");
        })
    };
    ($data:ident : $data_ty:ty , $($arg:tt)*) => {
        Text::new(move | $data : $data_ty , label: &mut String | {
            use std::fmt::Write;
            label.clear();
            write!(label, $($arg)*)
                .expect("format_text: formatting error");
        })
    };
}
