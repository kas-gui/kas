// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simplified text label
//!
//! This module supports text strings in macros.
//! Direct usage of this module from user code is not supported.

use crate::class::HasStr;
use crate::event::ConfigMgr;
use crate::geom::Rect;
use crate::layout::{Align, AlignHints, AxisInfo, SizeRules};
use crate::text::{Text, TextApi};
use crate::theme::{DrawMgr, SizeMgr, TextClass};
use crate::{Layout, WidgetCore};
use kas_macros::impl_scope;

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
        pub const CLASS: TextClass = TextClass::Label(true);
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            size_mgr.text_bound(&mut self.label, Self::CLASS, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let align = align.unwrap_or(Align::Default, Align::Center);
            mgr.text_set_size(&mut self.label, Self::CLASS, rect.size, align);
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.text(self.rect().pos, &self.label, Self::CLASS);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }
}