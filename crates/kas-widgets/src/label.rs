// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Fixed text widgets

use super::adapt::MapAny;
use kas::prelude::*;
use kas::text::format::{EditableText, FormattableText};
use kas::text::Text;
use kas::theme::TextClass;

/// Construct a [`Label`] which accepts any data
#[inline]
pub fn label<A, T: FormattableText + 'static>(label: T) -> MapAny<A, Label<T>> {
    MapAny::new(Label::new(label))
}

impl_scope! {
    /// A text label
    ///
    /// `Label` derives its contents from input data. Use [`Text`](crate::Text)
    /// instead for fixed contents.
    ///
    /// A text label. Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled by default.
    ///
    /// This type is generic over the text type.
    /// See also: [`StrLabel`], [`StringLabel`], [`AccelLabel`].
    #[impl_default(where T: Default)]
    #[derive(Clone, Debug)]
    #[widget {
        Data = ();
    }]
    pub struct Label<T: FormattableText + 'static> {
        core: widget_core!(),
        class: TextClass = TextClass::Label(true),
        label: Text<T>,
    }

    impl Self {
        /// Construct from `label`
        #[inline]
        pub fn new(label: T) -> Self {
            Label {
                core: Default::default(),
                class: TextClass::Label(true),
                label: Text::new(label),
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
        pub fn text(&self) -> &Text<T> {
            &self.label
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: T) -> Action {
            match self.label.set_and_try_prepare(text) {
                Ok(true) => Action::RESIZE,
                _ => Action::REDRAW,
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Default, Align::Center);
            sizer.text_rules(&mut self.label, self.class, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            cx.text_set_size(&mut self.label, self.class, rect.size, None);
        }

        #[cfg(feature = "min_spec")]
        default fn draw(&mut self, mut draw: DrawCx) {
            draw.text_effects(self.rect(), &self.label, self.class);
        }
        #[cfg(not(feature = "min_spec"))]
        fn draw(&mut self, mut draw: DrawCx) {
            draw.text_effects(self.rect(), &self.label, self.class);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl HasString for Self
    where
        T: EditableText,
    {
        fn set_string(&mut self, string: String) -> Action {
            self.label.set_string(string);
            match self.label.try_prepare() {
                Ok(true) => Action::RESIZE,
                _ => Action::REDRAW,
            }
        }
    }
}

// Str/String representations have no effects, so use simpler draw call
#[cfg(feature = "min_spec")]
impl<'a> Layout for Label<&'a str> {
    fn draw(&mut self, mut draw: DrawCx) {
        draw.text(self.rect(), &self.label, self.class);
    }
}
#[cfg(feature = "min_spec")]
impl Layout for StringLabel {
    fn draw(&mut self, mut draw: DrawCx) {
        draw.text(self.rect(), &self.label, self.class);
    }
}

/* TODO(specialization): can we support this? min_specialization is not enough.
impl<U, T: From<U> + FormattableText + 'static> From<U> for Label<T> {
    default fn from(text: U) -> Self {
        let text = T::from(text);
        Label::new(text)
    }
}*/

impl<T: FormattableText + 'static> From<T> for Label<T> {
    fn from(label: T) -> Self {
        Label::new(label)
    }
}

impl<'a> From<&'a str> for Label<String> {
    fn from(label: &'a str) -> Self {
        Label::new(label.to_string())
    }
}

/// Label with `&'static str` as backing type
///
/// Warning: this type does not support [`HasString`]. Assignment is possible
/// via [`Label::set_text`], but only for `&'static str`, so most of the time
/// [`StringLabel`] will be preferred when assignment is required.
/// (Also note that the overhead of allocating and copying a `String` is
/// irrelevant considering those used for text layout and drawing.)
pub type StrLabel = Label<&'static str>;

/// Label with `String` as backing type
pub type StringLabel = Label<String>;

// NOTE: AccelLabel requires a different text class. Once specialization is
// stable we can simply replace the `draw` method, but for now we use a whole
// new type.
impl_scope! {
    /// A label supporting an accelerator key
    ///
    /// An `AccelLabel` is a variant of [`Label`] supporting [`AccelString`],
    /// for example "&Edit" binds an action to <kbd>Alt+E</kbd>. When the
    /// corresponding key-sequence is pressed this widget sends the message
    /// [`kas::message::Activate`] which should be handled by a parent.
    ///
    /// A text label. Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled by default.
    #[impl_default]
    #[derive(Clone, Debug)]
    #[widget]
    pub struct AccelLabel {
        core: widget_core!(),
        class: TextClass = TextClass::Label(true),
        label: Text<AccelString>,
    }

    impl Self {
        /// Construct from `label`
        #[inline]
        pub fn new(label: impl Into<AccelString>) -> Self {
            AccelLabel {
                core: Default::default(),
                class: TextClass::AccelLabel(true),
                label: Text::new(label.into()),
            }
        }

        /// Get text class
        #[inline]
        pub fn class(&self) -> TextClass {
            self.class
        }

        /// Set text class
        ///
        /// Default: `AccelLabel::Label(true)`
        #[inline]
        pub fn set_class(&mut self, class: TextClass) {
            self.class = class;
        }

        /// Set text class (inline)
        ///
        /// Default: `AccelLabel::Label(true)`
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
        /// This is equivalent to `label.set_class(TextClass::AccelLabel(wrap))`.
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.class = TextClass::AccelLabel(wrap);
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.class = TextClass::Label(wrap);
            self
        }

        /// Get read access to the text object
        #[inline]
        pub fn text(&self) -> &Text<AccelString> {
            &self.label
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: AccelString) -> Action {
            match self.label.set_and_try_prepare(text) {
                Ok(true) => Action::RESIZE,
                _ => Action::REDRAW,
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Default, Align::Center);
            sizer.text_rules(&mut self.label, self.class, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            cx.text_set_size(&mut self.label, self.class, rect.size, None);
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.text_effects(self.rect(), &self.label, self.class);
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            if let Some(key) = self.label.text().key() {
                cx.add_accel_key(self.id_ref(), key.clone());
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> Response {
            match event {
                Event::Command(cmd) if cmd.is_activate() => {
                    cx.push(kas::message::Activate);
                    Response::Used
                }
                _ => Response::Unused
            }
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl SetAccel for AccelLabel {
        fn set_accel_string(&mut self, string: AccelString) -> Action {
            if self.label.text().key() != string.key() {
                return Action::RECONFIGURE;
            }
            self.set_text(string)
        }
    }
}
