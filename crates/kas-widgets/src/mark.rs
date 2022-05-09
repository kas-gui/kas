// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Mark widget

use kas::layout::{AxisInfo, SizeRules};
use kas::theme::{DrawMgr, MarkStyle, SizeMgr};
use kas::Layout;
use kas_macros::impl_scope;

impl_scope! {
    /// A mark
    #[derive(Clone, Debug)]
    #[widget]
    pub struct Mark {
        core: widget_core!(),
        style: MarkStyle,
    }
    impl Self {
        /// Construct
        pub fn new(style: MarkStyle) -> Self {
            Mark {
                core: Default::default(),
                style,
            }
        }

        /// Get mark style
        #[inline]
        pub fn mark(&self) -> MarkStyle {
            self.style
        }

        /// Set mark style
        #[inline]
        pub fn set_mark(&mut self, mark: MarkStyle) {
            self.style = mark;
        }
    }
    impl Layout for Self {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            mgr.mark(self.style, axis)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.mark(self.core.rect, self.style);
        }
    }
}
