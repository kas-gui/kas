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
    /// The label supports access keys; on usage the inner widget will receive
    /// event `Event::Command(Command::Activate)`.
    ///
    /// Mouse/touch input on the label sends events to the inner widget.
    #[derive(Clone, Default)]
    #[widget]
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
        pub fn label_direction(&self) -> Direction {
            self.dir.as_direction()
        }

        /// Take inner
        #[inline]
        pub fn take_inner(self) -> W {
            self.inner
        }

        /// Get whether line-wrapping is enabled
        #[inline]
        pub fn label_wrap(&self) -> bool {
            self.label.wrap()
        }

        /// Enable/disable line wrapping
        ///
        /// By default this is enabled.
        #[inline]
        pub fn set_label_wrap(&mut self, wrap: bool) {
            self.label.set_wrap(wrap);
        }

        /// Enable/disable line wrapping (inline)
        #[inline]
        pub fn with_label_wrap(mut self, wrap: bool) -> Self {
            self.label.set_wrap(wrap);
            self
        }

        /// Set label text
        ///
        /// Note: this must not be called before fonts have been initialised
        /// (usually done by the theme when the main loop starts).
        pub fn set_label_text<T: Into<AccessString>>(&mut self, cx: &mut EventState, text: T) {
            self.label.set_text(cx, text.into());
        }
    }

    impl Tile for Self {
        fn role_child_properties(&self, cx: &mut dyn RoleCx, index: usize) {
            if index == widget_index!(self.inner) {
                cx.set_label(self.label.id());
            }
        }

        fn nav_next(&self, _: bool, from: Option<usize>) -> Option<usize> {
            from.xor(Some(widget_index!(self.inner)))
        }

        fn probe(&self, _: Coord) -> Id {
            self.inner.id()
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.make_child_id(widget_index!(self.inner));
            if id.is_valid() {
                cx.configure(self.inner.as_node(data), id);
            }

            let id = self.make_child_id(widget_index!(self.label));
            if id.is_valid() {
                cx.configure(self.label.as_node(&()), id);
                self.label.set_target(self.inner.id());
            }
        }
    }
}

#[impl_self]
mod WithHiddenLabel {
    /// A wrapper widget with a hidden label
    ///
    /// This label is not normally visible but may be read by accessibility
    /// tools and tooltips.
    #[derive(Clone, Default)]
    #[derive_widget]
    pub struct WithHiddenLabel<W: Widget> {
        #[widget]
        inner: W,
        label: String,
    }

    impl Self {
        /// Wrap `inner`, adding a hidden `label`
        #[inline]
        pub fn new<T: ToString>(inner: W, label: T) -> Self {
            WithHiddenLabel {
                inner,
                label: label.to_string(),
            }
        }

        /// Take inner
        #[inline]
        pub fn take_inner(self) -> W {
            self.inner
        }

        /// Set the label
        pub fn set_label<T: ToString>(&mut self, text: T) {
            self.label = text.to_string();
        }
    }

    impl Tile for Self {
        fn tooltip(&self) -> Option<&str> {
            Some(&self.label)
        }

        fn role(&self, cx: &mut dyn RoleCx) -> Role<'_> {
            cx.set_label(&self.label);
            self.inner.role(cx)
        }
    }
}
