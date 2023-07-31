// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Progress bar

use kas::prelude::*;
use kas::theme::Feature;

impl_scope! {
    /// A progress bar
    ///
    /// The "progress" value may range from 0.0 to 1.0.
    #[autoimpl(Debug ignore self.value_fn)]
    #[widget]
    pub struct ProgressBar<A, D: Directional> {
        core: widget_core!(),
        align: AlignPair,
        direction: D,
        value: f32,
        value_fn: Box<dyn Fn(&ConfigCx, &A) -> f32>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a progress bar
        ///
        /// Closure `value_fn` returns the current progress as a value between
        /// 0.0 and 1.0.
        #[inline]
        pub fn new(value_fn: impl Fn(&ConfigCx, &A) -> f32 + 'static) -> Self {
            ProgressBar::new_with_direction(value_fn, D::default())
        }
    }

    impl<A>  ProgressBar<A, kas::dir::Right> {
        /// Construct a progress bar (horizontal)
        ///
        /// Closure `value_fn` returns the current progress as a value between
        /// 0.0 and 1.0.
        #[inline]
        pub fn right(value_fn: impl Fn(&ConfigCx, &A) -> f32 + 'static) -> Self {
            ProgressBar::new(value_fn)
        }
    }

    impl Self {
        /// Construct a slider with the given `direction`
        ///
        /// Closure `value_fn` returns the current progress as a value between
        /// 0.0 and 1.0.
        #[inline]
        pub fn new_with_direction(value_fn: impl Fn(&ConfigCx, &A) -> f32 + 'static, direction: D) -> Self {
            ProgressBar {
                core: Default::default(),
                align: Default::default(),
                direction,
                value: 0.0,
                value_fn: Box::new(value_fn),
            }
        }

        /// Get the progress bar's direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.align.set_component(
                axis,
                match axis.is_vertical() == self.direction.is_vertical() {
                    false => axis.align_or_center(),
                    true => axis.align_or_stretch(),
                },
            );
            size_mgr.feature(Feature::ProgressBar(self.direction()), axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            let rect = cx.align_feature(Feature::ProgressBar(self.direction()), rect, self.align);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let dir = self.direction.as_direction();
            draw.progress_bar(self.rect(), dir, self.value);
        }
    }

    impl Events for Self {
        type Data = A;

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            let value = (self.value_fn)(cx, data);
            self.value = value.clamp(0.0, 1.0);
        }
    }
}
