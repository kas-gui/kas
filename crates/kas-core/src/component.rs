// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget components

use crate::geom::{Coord, Rect};
use crate::layout::{AlignHints, AxisInfo, SetRectMgr, SizeRules};
use crate::theme::{DrawMgr, MarkStyle, SizeMgr};
use crate::{Layout, WidgetId};
use kas_macros::impl_scope;

impl_scope! {
    /// A mark
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Mark {
        pub style: MarkStyle,
        pub rect: Rect,
    }
    impl Self {
        /// Construct
        pub fn new(style: MarkStyle) -> Self {
            let rect = Rect::ZERO;
            Mark { style, rect }
        }
    }
    impl Layout for Self {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            mgr.mark(self.style, axis)
        }

        fn set_rect(&mut self, _: &mut SetRectMgr, rect: Rect, _: AlignHints) {
            self.rect = rect;
        }

        fn find_id(&mut self, _: Coord) -> Option<WidgetId> {
            None
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.mark(self.rect, self.style);
        }
    }
}
