// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A separator

use crate::menu::Menu;
use kas::prelude::*;
use std::marker::PhantomData;

#[impl_self]
mod Separator {
    /// A separator
    ///
    /// This widget draws a bar when in a list.
    #[autoimpl(Clone, Debug, Default)]
    #[widget]
    pub struct Separator<A> {
        core: widget_core!(),
        _pd: PhantomData<A>,
    }

    impl Self {
        /// Construct a frame, with void message type
        #[inline]
        pub fn new() -> Self {
            Separator {
                core: Default::default(),
                _pd: PhantomData,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            cx.feature(kas::theme::Feature::Separator, axis)
        }

        fn draw(&self, mut draw: DrawCx) {
            draw.separator(self.rect());
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Border
        }
    }

    impl Events for Self {
        type Data = A;
    }

    /// A separator is a valid menu widget
    impl Menu for Self {}
}
