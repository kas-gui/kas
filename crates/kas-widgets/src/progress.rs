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
    pub struct ProgressBar<A, D: Directional = kas::dir::Right> {
        core: widget_core!(),
        direction: D,
        value: f32,
        value_fn: Box<dyn Fn(&ConfigCx, &A) -> f32>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a slider
        ///
        /// Closure `value_fn` returns the current progress as a value between
        /// 0.0 and 1.0.
        #[inline]
        pub fn new(value_fn: impl Fn(&ConfigCx, &A) -> f32 + 'static) -> Self {
            Self::new_dir(value_fn, D::default())
        }
    }
    impl<A> ProgressBar<A, kas::dir::Right> {
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
        pub fn new_dir(value_fn: impl Fn(&ConfigCx, &A) -> f32 + 'static, direction: D) -> Self {
            ProgressBar {
                core: Default::default(),
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
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            sizer.feature(Feature::ProgressBar(self.direction()), axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = match self.direction.is_vertical() {
                false => AlignPair::new(Align::Stretch, hints.vert.unwrap_or(Align::Center)),
                true => AlignPair::new(hints.horiz.unwrap_or(Align::Center), Align::Stretch),
            };
            let rect = cx.align_feature(Feature::ProgressBar(self.direction()), rect, align);
            widget_set_rect!(rect);
        }

        fn draw(&mut self, mut draw: DrawCx) {
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
