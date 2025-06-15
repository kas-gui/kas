// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Fixed text widgets

use super::adapt::MapAny;
use kas::prelude::*;
use kas::text::format::FormattableText;
use kas::theme::{Text, TextClass};

#[impl_self]
mod Label {
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
    #[derive(Clone, Debug, Default)]
    #[widget]
    #[layout(self.text)]
    pub struct Label<T: FormattableText + 'static> {
        core: widget_core!(),
        text: Text<T>,
    }

    impl Self {
        /// Construct from `text`
        #[inline]
        pub fn new(text: T) -> Self {
            Label {
                core: Default::default(),
                text: Text::new(text, TextClass::Label(true)),
            }
        }

        /// Construct from `text`, mapping to support any data type
        #[inline]
        pub fn new_any<A>(text: T) -> MapAny<A, Self> {
            MapAny::new(Label::new(text))
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
        pub fn text(&self) -> &Text<T> {
            &self.text
        }

        /// Set text in an existing `Label`
        pub fn set_text(&mut self, cx: &mut EventState, text: T) {
            self.text.set_text(text);
            let act = self.text.reprepare_action();
            cx.action(self, act);
        }

        /// Get text contents
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

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }
    }

    impl Label<String> {
        /// Set text contents from a string
        pub fn set_string(&mut self, cx: &mut EventState, string: String) {
            if self.text.set_string(string) {
                cx.action(self.id(), self.text.reprepare_action());
            }
        }
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
    fn from(text: T) -> Self {
        Label::new(text)
    }
}

impl<'a> From<&'a str> for Label<String> {
    fn from(text: &'a str) -> Self {
        Label::new(text.to_string())
    }
}

// NOTE: AccessLabel requires a different text class. Once specialization is
// stable we can simply replace the `draw` method, but for now we use a whole
// new type.
#[impl_self]
mod AccessLabel {
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
    #[derive(Clone, Debug, Default)]
    #[widget]
    #[layout(self.text)]
    pub struct AccessLabel {
        core: widget_core!(),
        text: Text<AccessString>,
    }

    impl Self {
        /// Construct from `text`
        #[inline]
        pub fn new(text: impl Into<AccessString>) -> Self {
            AccessLabel {
                core: Default::default(),
                text: Text::new(text.into(), TextClass::AccessLabel(true)),
            }
        }

        /// Get text class
        #[inline]
        pub fn class(&self) -> TextClass {
            self.text.class()
        }

        /// Set text class
        ///
        /// Default: `AccessLabel::Label(true)`
        #[inline]
        pub fn set_class(&mut self, class: TextClass) {
            self.text.set_class(class);
        }

        /// Set text class (inline)
        ///
        /// Default: `AccessLabel::Label(true)`
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
        /// This is equivalent to `label.set_class(TextClass::AccessLabel(wrap))`.
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.text.set_class(TextClass::AccessLabel(wrap));
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.text.set_class(TextClass::AccessLabel(wrap));
            self
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }

        /// Get read access to the text object
        #[inline]
        pub fn text(&self) -> &Text<AccessString> {
            &self.text
        }

        /// Set text in an existing `Label`
        pub fn set_text(&mut self, cx: &mut EventState, text: AccessString) {
            self.text.set_text(text);
            let act = self.text.reprepare_action();
            cx.action(self, act);
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.text
                .set_rect(cx, rect, hints.combine(AlignHints::VERT_CENTER));
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);

            if let Some(key) = self.text.text().key() {
                cx.add_access_key(self.id_ref(), key.clone());
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    cx.push(kas::messages::Activate(code));
                    Used
                }
                _ => Unused,
            }
        }
    }
}
