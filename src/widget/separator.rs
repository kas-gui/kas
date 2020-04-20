// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A separator

use std::fmt::Debug;
use std::marker::PhantomData;

use kas::draw::{DrawHandle, SizeHandle};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;

/// A separator
///
/// This widget draws a bar when in a list. It may expand larger than expected
/// if no other widget will fill spare space.
#[handler(msg=M)]
#[derive(Clone, Debug, Default, Widget)]
pub struct Separator<M: Debug> {
    #[widget_core]
    core: CoreData,
    _msg: PhantomData<M>,
}

impl Separator<event::VoidMsg> {
    /// Construct a frame, with void message type
    #[inline]
    pub fn new() -> Self {
        Separator {
            core: Default::default(),
            _msg: Default::default(),
        }
    }
}

impl<M: Debug> Separator<M> {
    /// Construct a frame, with inferred message type
    ///
    /// This may be useful when embedding a separator in a list with
    /// a given message type.
    #[inline]
    pub fn infer() -> Self {
        Separator {
            core: Default::default(),
            _msg: Default::default(),
        }
    }
}

impl<M: Debug> Layout for Separator<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        SizeRules::extract_fixed(axis.is_vertical(), size_handle.frame(), Default::default())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &event::ManagerState, _: bool) {
        draw_handle.separator(self.core.rect);
    }
}
