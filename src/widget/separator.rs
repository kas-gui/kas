// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A separator

use std::fmt::Debug;

use kas::draw::{DrawHandle, SizeHandle};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;

/// A separator
///
/// This widget draws a bar when in a list. It may expand larger than expected
/// if no other widget will fill spare space.
#[derive(Clone, Debug, Default, Widget)]
pub struct Separator {
    #[widget_core]
    core: CoreData,
}

impl Separator {
    /// Construct a frame
    #[inline]
    pub fn new() -> Self {
        Separator {
            core: Default::default(),
        }
    }
}

impl Layout for Separator {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        SizeRules::extract_fixed(axis.is_vertical(), size_handle.frame(), Default::default())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        draw_handle.separator(self.core.rect);
    }
}
