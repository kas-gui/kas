// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper adding a label

use kas::text::util::set_text_and_prepare;
use kas::theme::TextClass;
use kas::{event, layout, prelude::*};

widget! {
    /// A wrapper widget with a label
    ///
    /// The label supports accelerator keys, which activate `self.inner` on
    /// usage.
    ///
    /// Mouse/touch input on the label sends events to the inner widget.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[derive(Clone, Default, Debug)]
    #[handler(msg = W::Msg)]
    pub struct WithLabel<W: Widget, D: Directional> {
        #[widget_core]
        core: CoreData,
        dir: D,
        #[widget]
        inner: W,
        wrap: bool,
        layout_store: layout::FixedRowStorage<2>,
        label_store: layout::TextStorage,
        label: Text<AccelString>,
    }

    impl Self where D: Default {
        /// Construct from `inner` widget and `label`
        #[inline]
        pub fn new<T: Into<AccelString>>(inner: W, label: T) -> Self {
            Self::new_with_direction(D::default(), inner, label)
        }
    }

    impl Self {
        /// Construct from `direction`, `inner` widget and `label`
        #[inline]
        pub fn new_with_direction<T: Into<AccelString>>(direction: D, inner: W, label: T) -> Self {
            WithLabel {
                core: Default::default(),
                dir: direction,
                inner,
                wrap: true,
                layout_store: Default::default(),
                label_store: Default::default(),
                label: Text::new_multi(label.into()),
            }
        }

        /// Get the direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.dir.as_direction()
        }

        /// Deconstruct into `(inner, label)`
        #[inline]
        pub fn deconstruct(self) -> (W, Text<AccelString>) {
            (self.inner, self.label)
        }

        /// Get whether line-wrapping is enabled
        #[inline]
        pub fn wrap(&self) -> bool {
            self.wrap
        }

        /// Enable/disable line wrapping
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.wrap = wrap;
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.wrap = wrap;
            self
        }

        /// Set text in an existing `Label`
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text<T: Into<AccelString>>(&mut self, text: T) -> TkAction {
            set_text_and_prepare(&mut self.label, text.into(), self.core.rect.size)
        }

        /// Get the accelerator keys
        pub fn keys(&self) -> &[event::VirtualKeyCode] {
            self.label.text().keys()
        }
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.inner.id_ref(), self.keys());
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let arr = [
                layout::Layout::single(&mut self.inner),
                layout::Layout::text(&mut self.label_store, &mut self.label, TextClass::Label(self.wrap)),
            ];
            layout::Layout::list(arr.into_iter(), self.dir, &mut self.layout_store)
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            Some(self.inner.id())
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl SetAccel for Self {
        fn set_accel_string(&mut self, string: AccelString) -> TkAction {
            let mut action = TkAction::empty();
            if self.label.text().keys() != string.keys() {
                action |= TkAction::RECONFIGURE;
            }
            action | set_text_and_prepare(&mut self.label, string, self.core.rect.size)
        }
    }
}
