// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Fixed text widgets

use super::adapt::MapAny;
use kas::prelude::*;
use kas::text::format::{EditableText, FormattableText};
use kas::text::{NotReady, Text};
use kas::theme::TextClass;

/// Construct a [`Label`]
#[inline]
pub fn label<T: FormattableText + 'static>(label: T) -> Label<T> {
    Label::new(label)
}

/// Construct a [`Label`] which accepts any data
///
/// This is just a shortcut for `Label::new(text).map_any()`.
#[inline]
pub fn label_any<A, T: FormattableText + 'static>(label: T) -> MapAny<A, Label<T>> {
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
    /// See also: [`AccessLabel`].
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
            match self.label.set_and_prepare(text) {
                Err(NotReady) => Action::empty(),
                Ok(false) => Action::REDRAW,
                Ok(true) => Action::RESIZE,
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Default, Align::Center);
            sizer.text_rules(&mut self.label, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            cx.text_set_size(&mut self.label, rect.size, None);
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

    impl Events for Self {
        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.label, self.class);
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
            match self.label.prepare() {
                Err(NotReady) => Action::empty(),
                Ok(false) => Action::REDRAW,
                Ok(true) => Action::RESIZE,
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
impl Layout for Label<String> {
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

// NOTE: AccessLabel requires a different text class. Once specialization is
// stable we can simply replace the `draw` method, but for now we use a whole
// new type.
impl_scope! {
    /// A label supporting an access key
    ///
    /// An `AccessLabel` is a variant of [`Label`] supporting [`AccessString`],
    /// for example "&Edit" binds an action to <kbd>Alt+E</kbd>. When the
    /// corresponding key-sequence is pressed this widget sends the message
    /// [`kas::messages::Activate`] which should be handled by a parent.
    ///
    /// A text label. Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled by default.
    #[impl_default]
    #[derive(Clone, Debug)]
    #[widget]
    pub struct AccessLabel {
        core: widget_core!(),
        class: TextClass = TextClass::Label(true),
        label: Text<AccessString>,
    }

    impl Self {
        /// Construct from `label`
        #[inline]
        pub fn new(label: impl Into<AccessString>) -> Self {
            AccessLabel {
                core: Default::default(),
                class: TextClass::AccessLabel(true),
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
        /// Default: `AccessLabel::Label(true)`
        #[inline]
        pub fn set_class(&mut self, class: TextClass) {
            self.class = class;
        }

        /// Set text class (inline)
        ///
        /// Default: `AccessLabel::Label(true)`
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
        /// This is equivalent to `label.set_class(TextClass::AccessLabel(wrap))`.
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.class = TextClass::AccessLabel(wrap);
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.class = TextClass::Label(wrap);
            self
        }

        /// Get read access to the text object
        #[inline]
        pub fn text(&self) -> &Text<AccessString> {
            &self.label
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: AccessString) -> Action {
            match self.label.set_and_prepare(text) {
                Err(NotReady) => Action::empty(),
                Ok(false) => Action::REDRAW,
                Ok(true) => Action::RESIZE,
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Default, Align::Center);
            sizer.text_rules(&mut self.label, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            cx.text_set_size(&mut self.label, rect.size, None);
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.text_effects(self.rect(), &self.label, self.class);
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.label, self.class);

            if let Some(key) = self.label.text().key() {
                cx.add_access_key(self.id_ref(), key.clone());
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    cx.push(kas::messages::Activate(code));
                    Used
                }
                _ => Unused
            }
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }
}
