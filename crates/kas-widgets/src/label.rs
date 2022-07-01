// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use kas::text::format::{EditableText, FormattableText};
use kas::theme::TextClass;
use kas::{event, prelude::*};

impl_scope! {
    /// A text label
    ///
    /// A text label. Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled by default.
    ///
    /// This type is generic over the text type.
    /// See also: [`StrLabel`], [`StringLabel`], [`AccelLabel`].
    #[impl_default(where T: Default)]
    #[derive(Clone, Debug)]
    #[widget]
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
                label: Text::new_multi(label),
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

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: T) -> TkAction {
            kas::text::util::set_text_and_prepare(&mut self.label, text, self.core.rect.size)
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            size_mgr.text_bound(&mut self.label, self.class, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let align = align.unwrap_or(Align::Default, Align::Center);
            mgr.text_set_size(&mut self.label, self.class, rect.size, align);
        }

        #[cfg(feature = "min_spec")]
        default fn draw(&mut self, mut draw: DrawMgr) {
            draw.text_effects(self.rect().pos, &self.label, self.class);
        }
        #[cfg(not(feature = "min_spec"))]
        fn draw(&mut self, mut draw: DrawMgr) {
            draw.text_effects(self.rect().pos, &self.label, self.class);
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
        fn set_string(&mut self, string: String) -> TkAction {
            kas::text::util::set_string_and_prepare(&mut self.label, string, self.core.rect.size)
        }
    }
}

// Str/String representations have no effects, so use simpler draw call
#[cfg(feature = "min_spec")]
impl<'a> Layout for Label<&'a str> {
    fn draw(&mut self, mut draw: DrawMgr) {
        draw.text(self.rect().pos, &self.label, self.class);
    }
}
#[cfg(feature = "min_spec")]
impl Layout for StringLabel {
    fn draw(&mut self, mut draw: DrawMgr) {
        draw.text(self.rect().pos, &self.label, self.class);
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
    /// Accelerator keys are not useful on plain labels. To be useful, a parent
    /// widget must do something like:
    /// ```no_test
    /// impl Widget for Self {
    ///     fn configure(&mut self, mgr: &mut EventMgr) {
    ///         let target = self.id(); // widget receiving Event::Activate
    ///         mgr.add_accel_keys(target, self.label.keys());
    ///     }
    //// }
    /// ```
    #[derive(Clone, Debug)]
    #[widget{
        derive = self.0;
    }]
    pub struct AccelLabel(Label<AccelString>);

    impl Default for Self {
        fn default() -> Self {
            AccelLabel::new("")
        }
    }

    impl Self {
        /// Construct from `label`
        #[inline]
        pub fn new<S: Into<AccelString>>(label: S) -> Self {
            let label = label.into();
            AccelLabel(Label::new(label)).with_class(TextClass::AccelLabel(true))
        }

        /// Get text class
        #[inline]
        pub fn class(&self) -> TextClass {
            self.0.class
        }

        /// Set text class
        ///
        /// Default: `AccelLabel::Label(true)`
        #[inline]
        pub fn set_class(&mut self, class: TextClass) {
            self.0.set_class(class);
        }

        /// Set text class (inline)
        ///
        /// Default: `AccelLabel::Label(true)`
        #[inline]
        pub fn with_class(mut self, class: TextClass) -> Self {
            self.0.set_class(class);
            self
        }

        /// Get whether line-wrapping is enabled
        #[inline]
        pub fn wrap(&self) -> bool {
            self.0.wrap()
        }

        /// Enable/disable line wrapping
        ///
        /// This is equivalent to `label.set_class(AccelLabel::Label(wrap))`.
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.0.set_wrap(wrap);
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.0.set_wrap(wrap);
            self
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text(&mut self, text: AccelString) -> TkAction {
            kas::text::util::set_text_and_prepare(&mut self.0.label, text, self.0.core.rect.size)
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.0.size_rules(size_mgr, axis)
        }

        #[inline]
        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.0.set_rect(mgr, rect, align)
        }

        #[inline]
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.0.find_id(coord)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.text_effects(self.rect().pos, &self.0.label, self.0.class);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.0.label.as_str()
        }
    }

    impl AccelLabel {
        /// Get the accelerator keys
        pub fn keys(&self) -> &[event::VirtualKeyCode] {
            self.0.label.text().keys()
        }
    }

    impl SetAccel for AccelLabel {
        fn set_accel_string(&mut self, string: AccelString) -> TkAction {
            let mut action = TkAction::empty();
            if self.0.label.text().keys() != string.keys() {
                action |= TkAction::RECONFIGURE;
            }
            action | kas::text::util::set_text_and_prepare(&mut self.0.label, string, self.0.core.rect.size)
        }
    }
}
