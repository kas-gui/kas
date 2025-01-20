// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hidden extras
//!
//! It turns out that some widgets are needed in kas-core. This module is
//! hidden by default and direct usage (outside of kas crates) is
//! not supported (i.e. **changes are not considered breaking**).

use crate::classes::HasStr;
use crate::event::ConfigCx;
use crate::geom::Rect;
use crate::layout::{Align, AlignHints, AxisInfo, SizeRules};
use crate::theme::{DrawCx, SizeCx, Text, TextClass};
use crate::{Events, Layout, Tile, Widget};
use kas_macros::{autoimpl, impl_scope};

impl_scope! {
    /// A simple text label
    ///
    /// Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled.
    #[derive(Clone, Debug, Default)]
    #[widget {
        Data = ();
    }]
    pub struct StrLabel {
        core: widget_core!(),
        text: Text<&'static str>,
    }

    impl Self {
        /// Construct from `text`
        #[inline]
        pub fn new(text: &'static str) -> Self {
            StrLabel {
                core: Default::default(),
                text: Text::new(text, TextClass::Label(false)),
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            sizer.text_rules(&mut self.text, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.core.rect = rect;
            let align = hints.complete(Align::Default, Align::Center);
            cx.text_set_size(&mut self.text, rect.size, align);
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.text(self.rect(), &self.text);
        }
    }

    impl Events for Self {
        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.text.as_str()
        }
    }
}

impl_scope! {
    /// Map any input data to `()`
    ///
    /// This is a generic data-mapping widget-wrapper with fixed `()` input
    /// data type.
    ///
    /// This struct is a thin wrapper around the inner widget without its own
    /// [`Id`](crate::Id). It supports [`Deref`](std::ops::Deref) and
    /// [`DerefMut`](std::ops::DerefMut) to the inner widget.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    #[widget {
        Data = A;
        data_expr = &();
        derive = self.inner;
    }]
    pub struct MapAny<A, W: Widget<Data = ()>> {
        _a: std::marker::PhantomData<A>,
        /// The inner widget
        pub inner: W,
    }

    impl Self {
        /// Construct
        pub fn new(inner: W) -> Self {
            MapAny {
                _a: std::marker::PhantomData,
                inner,
            }
        }
    }
}
