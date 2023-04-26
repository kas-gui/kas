// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hidden extras
//!
//! It turns out that some widgets are needed in kas-core. This module is
//! hidden by default and direct usage (outside of kas crates) is
//! not supported (i.e. **changes are not considered breaking**).

use crate::class::HasStr;
use crate::event::ConfigMgr;
use crate::geom::Rect;
use crate::layout::{Align, AxisInfo, SizeRules};
use crate::text::{Text, TextApi};
use crate::theme::{DrawMgr, SizeMgr, TextClass};
use crate::{Layout, Widget, WidgetCore};
use kas_macros::{autoimpl, impl_scope};
use std::marker::PhantomData;

impl_scope! {
    /// Data adaptation: map to ()
    #[widget {
        data = A;
        layout = self.inner;
    }]
    #[autoimpl(Debug)]
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[doc(hidden)]
    pub struct Discard<A, W: Widget<Data = ()>> {
        core: widget_core!(),
        #[widget(&())]
        inner: W,
        _data: PhantomData<A>,
    }

    impl Self {
        /// Construct
        pub fn new(inner: W) -> Self {
            Discard {
                core: Default::default(),
                inner,
                _data: PhantomData,
            }
        }
    }
}

impl_scope! {
    /// A simple text label
    ///
    /// Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled.
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct StrLabel {
        core: widget_core!(),
        label: Text<&'static str>,
    }

    impl Self {
        /// Construct from `label`
        #[inline]
        pub fn new(label: &'static str) -> Self {
            StrLabel {
                core: Default::default(),
                label: Text::new(label),
            }
        }

        /// Text class
        pub const CLASS: TextClass = TextClass::Label(false);
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Default, Align::Center);
            size_mgr.text_rules(&mut self.label, Self::CLASS, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;
            mgr.text_set_size(&mut self.label, Self::CLASS, rect.size, None);
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.text(self.rect(), &self.label, Self::CLASS);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }
}
