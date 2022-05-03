// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A separator

use std::fmt::Debug;

use crate::menu::Menu;
use kas::prelude::*;

impl_scope! {
    /// A separator
    ///
    /// This widget draws a bar when in a list.
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct Separator {
        #[widget_core]
        core: CoreData,
    }

    impl Self {
        /// Construct a frame, with void message type
        #[inline]
        pub fn new() -> Self {
            Separator {
                core: Default::default(),
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            SizeRules::extract_fixed(axis, size_mgr.separator(), Margins::ZERO)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.separator(self.rect());
        }
    }

    /// A separator is a valid menu widget
    impl Menu for Self {}
}
