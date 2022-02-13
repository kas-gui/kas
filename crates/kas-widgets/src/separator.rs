// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A separator

use std::fmt::Debug;
use std::marker::PhantomData;

use crate::Menu;
use kas::{event::VoidMsg, prelude::*};

widget! {
    /// A separator
    ///
    /// This widget draws a bar when in a list.
    #[derive(Clone, Debug, Default)]
    #[handler(msg=M)]
    pub struct Separator<M: Debug + 'static = VoidMsg> {
        #[widget_core]
        core: CoreData,
        _msg: PhantomData<M>,
    }

    impl Separator<VoidMsg> {
        /// Construct a frame, with void message type
        #[inline]
        pub fn new() -> Self {
            Separator {
                core: Default::default(),
                _msg: Default::default(),
            }
        }
    }

    impl Self {
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

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            SizeRules::extract_fixed(axis, size_mgr.separator(), Margins::ZERO)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let mut draw = draw.with_core(self.core_data());
            draw.separator(self.core.rect);
        }
    }

    /// A separator is a valid menu widget
    impl Menu for Self {}
}
