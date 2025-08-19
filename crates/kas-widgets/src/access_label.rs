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
    /// This widget attempts to bind itself to its access key unless
    /// [a different target is set](Self::set_target). If the binding succeeds
    /// and the access key is used, the target will receive navigation focus
    /// (if supported; otherwise the first supporting ancestor is focussed) and
    /// `Event::Command(Command::Activate)` (likewise, an ancestor may handle
    /// the event). This `AccessLabel` does not support focus and will not
    /// handle the [`Command::Activate`] event.
    ///
    /// Alternatively, the parent of this widget may attempt to bind the access
    /// key ([`Self::access_key`]) using [`DrawCx::access_key`].
    #[derive(Clone, Debug, Default)]
    #[widget]
    #[layout(self.text)]
    pub struct AccessLabel {
        core: widget_core!(),
        target: Id,
        text: Text<AccessString>,
    }

    impl Self {
        /// Construct from `text`
        #[inline]
        pub fn new(text: impl Into<AccessString>) -> Self {
            AccessLabel {
                core: Default::default(),
                target: Default::default(),
                text: Text::new(text.into(), TextClass::AccessLabel(true)),
            }
        }

        /// Set the access key target
        ///
        /// This method should be called from [`Events::configure`] or
        /// [`Events::configure_recurse`].
        #[inline]
        pub fn set_target(&mut self, target: Id) {
            self.target = target;
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
        pub fn access_key(&self) -> Option<&Key> {
            self.text.text().key()
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.text
                .set_rect(cx, rect, hints.combine(AlignHints::VERT_CENTER));
        }

        fn draw(&self, mut draw: DrawCx) {
            if let Some(key) = self.text.text().key()
                && draw.access_key(&self.target, key)
            {
                draw.text(self.text.rect(), &self.text);
            } else {
                // draw without underline effects
                draw.text_with_effects(self.text.rect(), &self.text, &[]);
            }
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
            self.target = self.id();
            cx.text_configure(&mut self.text);
        }
    }
}
