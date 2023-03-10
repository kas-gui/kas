// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper adding a label

use crate::AccelLabel;
use kas::{layout, prelude::*};

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
        layout = list(self.dir) 'row: [self.inner, non_navigable: self.label];
    }]
    pub struct WithLabel<W: Widget, D: Directional> {
        core: widget_core!(),
        dir: D,
        #[widget]
        inner: W,
        #[widget]
        label: AccelLabel,
    }

    impl Self
    where
        D: Default,
    {
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
                label: AccelLabel::new(label.into()),
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
            &mut self.core.row
        }

        /// Get whether line-wrapping is enabled
        #[inline]
        pub fn wrap(&self) -> bool {
            self.label.wrap()
        }

        /// Enable/disable line wrapping
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_wrap(&mut self, wrap: bool) {
            self.label.set_wrap(wrap);
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_wrap(mut self, wrap: bool) -> Self {
            self.label.set_wrap(wrap);
            self
        }

        /// Set text
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_text<T: Into<AccelString>>(&mut self, text: T) -> Action {
            self.label.set_text(text.into())
        }
    }

    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.inner.id())
        }
    }

    impl SetAccel for Self {
        #[inline]
        fn set_accel_string(&mut self, string: AccelString) -> Action {
            self.label.set_accel_string(string)
        }
    }
}
