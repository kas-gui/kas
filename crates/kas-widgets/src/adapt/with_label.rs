// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper adding a label

use crate::AccessLabel;
use kas::prelude::*;

#[impl_self]
mod WithLabel {
    /// A wrapper widget with a label
    ///
    /// The label supports access keys, which activate `self.inner` on
    /// usage.
    ///
    /// Mouse/touch input on the label sends events to the inner widget.
    #[derive(Clone, Default)]
    #[widget(type Data = W::Data)]
    #[layout(list![self.inner, self.label].with_direction(self.dir))]
    pub struct WithLabel<W: Widget, D: Directional = Direction> {
        core: widget_core!(),
        dir: D,
        #[widget]
        inner: W,
        #[widget(&())]
        label: AccessLabel,
    }

    impl Self {
        /// Construct a wrapper around `inner` placing a `label` in the given `direction`
        pub fn new<T: Into<AccessString>>(inner: W, label: T) -> Self
        where
            D: Default,
        {
            Self::new_dir(inner, D::default(), label)
        }
    }
    impl<W: Widget> WithLabel<W, kas::dir::Left> {
        /// Construct from `inner` widget and `label`
        pub fn left<T: Into<AccessString>>(inner: W, label: T) -> Self {
            Self::new(inner, label)
        }
    }
    impl<W: Widget> WithLabel<W, kas::dir::Right> {
        /// Construct from `inner` widget and `label`
        pub fn right<T: Into<AccessString>>(inner: W, label: T) -> Self {
            Self::new(inner, label)
        }
    }

    impl Self {
        /// Construct a wrapper around `inner` placing a `label` in the given `direction`
        #[inline]
        pub fn new_dir<T: Into<AccessString>>(inner: W, direction: D, label: T) -> Self {
            WithLabel {
                core: Default::default(),
                dir: direction,
                inner,
                label: AccessLabel::new(label.into()),
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
        pub fn set_text<T: Into<AccessString>>(&mut self, cx: &mut EventState, text: T) {
            self.label.set_text(cx, text.into());
        }
    }

    impl Tile for Self {
        fn nav_next(&self, _: bool, from: Option<usize>) -> Option<usize> {
            from.xor(Some(widget_index!(self.inner)))
        }

        fn probe(&self, _: Coord) -> Id {
            self.inner.id()
        }
    }
}
