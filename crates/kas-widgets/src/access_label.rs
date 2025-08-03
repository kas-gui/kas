// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Label with access key

#[allow(unused)] use super::Label;
use kas::event::Key;
use kas::prelude::*;
use kas::theme::{Text, TextClass};

// NOTE: AccessLabel requires a different text class. Once specialization is
// stable we can simply replace the `draw` method, but for now we use a whole
// new type.
#[impl_self]
mod AccessLabel {
    /// A label supporting an access key
    ///
    /// An `AccessLabel` is a variant of [`Label`] supporting an access key,
    /// for example "&Edit" binds an action to <kbd>Alt+E</kbd> since by default
    /// <kbd>Alt</kbd> must be held to use access keys.
    /// The access key is parsed from the input `text` (see [`AccessString`])
    /// and underlined when <kbd>Alt</kbd> is held.
    ///
    /// Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled by default.
    ///
    /// ### Action bindings
    ///
    /// The access key may be registered explicitly by calling
    /// [`EventState::add_access_key`] using [`Self::access_key`].
    ///
    /// A parent widget (e.g. a push-button) registering itself as recipient of
    /// the access key is mostly equivalent to allowing the `AccessLabel` to
    /// register itself handler of its access key. Note that `AccessLabel` will
    /// attempt to register itself but fail if another widget registers itself
    /// first. `AccessLabel` will however not handle any events, thus an
    /// ancestor should handle `Event::Command(Command::Activate)` and
    /// navigation focus.
    ///
    /// A parent widget may register a different child (sibling of the
    /// `AccessLabel`) as handler of access key. This is complicated since (a)
    /// the registration must be made before the `AccessLabel` configures itself
    /// and (b) the [`Id`] of the sibling widget must be known. This can still
    /// be achieved using a custom [`Events::configure_recurse`] implementation;
    /// see for example the implementation of [`crate::CheckButton`].
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

        /// Get this label's access key, if any
        ///
        /// This key is parsed from the text.
        pub fn access_key(&self) -> Option<Key> {
            self.text.text().key()
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
            if let Some(key) = self.text.text().key() {
                Role::AccessLabel(self.text.as_str(), key.clone())
            } else {
                Role::Label(self.text.as_str())
            }
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
    }
}
