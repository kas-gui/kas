// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A separator

use crate::menu::Menu;
use kas::prelude::*;
use kas::theme::BarStyle;
use std::marker::PhantomData;

#[impl_self]
mod Separator {
    /// A separator
    ///
    /// This widget is drawn as a bar with automatically-determined direction.
    #[autoimpl(Debug, Default)]
    #[widget]
    pub struct Separator<A> {
        core: widget_core!(),
        _a: PhantomData<fn(A)>,
    }

    impl Self {
        /// Construct a frame, with void message type
        #[inline]
        pub fn new() -> Self {
            Separator {
                core: Default::default(),
                _a: PhantomData,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &SizeCx, axis: AxisInfo) -> SizeRules {
            cx.bar(BarStyle::Separator, axis.as_direction(), axis)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, _: AlignHints) {
            let dir = if rect.size.0 > rect.size.1 {
                Direction::Right
            } else {
                Direction::Down
            };
            let rect = cx.align_bar(BarStyle::Separator, dir, rect);
            self.core.set_rect(rect);
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
