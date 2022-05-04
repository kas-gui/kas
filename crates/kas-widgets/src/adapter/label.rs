// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper adding a label

use kas::component::Label;
use kas::theme::TextClass;
use kas::{event, layout, prelude::*};

impl_scope! {
    /// A wrapper widget with a label
    ///
    /// The label supports accelerator keys, which activate `self.inner` on
    /// usage.
    ///
    /// Mouse/touch input on the label sends events to the inner widget.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[derive(Clone, Default, Debug)]
    #[widget {
        layout = list(self.dir): [self.inner, component self.label];
    }]
    pub struct WithLabel<W: Widget, D: Directional> {
        core: widget_core!(),
        dir: D,
        #[widget]
        inner: W,
        wrap: bool,
        layout_store: layout::FixedRowStorage<2>,
        label: Label<AccelString>,
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
            let wrap = true;
            WithLabel {
                core: Default::default(),
                dir: direction,
                inner,
                wrap,
                layout_store: Default::default(),
                label: Label::new(label.into(), TextClass::Label(wrap)),
            }
        }

        /// Get the direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.dir.as_direction()
        }

        /// Take inner
        #[inline]
        pub fn take_inner(self) -> W {
            self.inner
        }

        /// Access layout storage
        ///
        /// The number of columns/rows is fixed at two: the `inner` widget, and
        /// the `label` (in this order, regardless of direction).
        #[inline]
        pub fn layout_storage(&mut self) -> &mut impl layout::RowStorage {
            &mut self.layout_store
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
            self.label.set_text_and_prepare(text.into(), self.core.rect.size)
        }

        /// Get the accelerator keys
        pub fn keys(&self) -> &[event::VirtualKeyCode] {
            self.label.keys()
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.inner.id_ref(), self.keys());
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.inner.id())
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
            if self.label.keys() != string.keys() {
                action |= TkAction::RECONFIGURE;
            }
            action | self.label.set_text_and_prepare(string, self.core.rect.size)
        }
    }
}
