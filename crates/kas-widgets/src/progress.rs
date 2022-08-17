// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Progress bar

use std::fmt::Debug;

use kas::prelude::*;
use kas::theme::Feature;

impl_scope! {
    /// A progress bar
    ///
    /// The "progress" value may range from 0.0 to 1.0.
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct ProgressBar<D: Directional> {
        core: widget_core!(),
        align: AlignPair,
        direction: D,
        value: f32,
    }

    impl Self where D: Default {
        /// Construct a progress bar
        ///
        /// The initial value is `0.0`; use `ProgressBar::with_value` to override.
        #[inline]
        pub fn new() -> Self {
            ProgressBar::new_with_direction(D::default())
        }
    }

    impl Self {
        /// Construct a slider with the given `direction`
        ///
        /// The initial value is `0.0`; use `ProgressBar::with_value` to override.
        #[inline]
        pub fn new_with_direction(direction: D) -> Self {
            ProgressBar {
                core: Default::default(),
                align: Default::default(),
                direction,
                value: 0.0,
            }
        }

        /// Get the progress bar's direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set the initial value
        #[inline]
        #[must_use]
        pub fn with_value(mut self, value: f32) -> Self {
            self.value = value.max(0.0).min(1.0);
            self
        }

        /// Get the current value
        #[inline]
        pub fn value(&self) -> f32 {
            self.value
        }

        /// Set the value
        ///
        /// Returns [`TkAction::REDRAW`] if a redraw is required.
        #[allow(clippy::float_cmp)]
        pub fn set_value(&mut self, value: f32) -> TkAction {
            let value = value.max(0.0).min(1.0);
            if value == self.value {
                TkAction::empty()
            } else {
                self.value = value;
                TkAction::REDRAW
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.align.set_component(axis, match axis.is_vertical() == self.direction.is_vertical() {
                false => axis.align_or_center(),
                true => axis.align_or_stretch(),
            });
            size_mgr.feature(Feature::ProgressBar(self.direction()), axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            let rect = mgr.align_feature(Feature::ProgressBar(self.direction()), rect, self.align);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let dir = self.direction.as_direction();
            draw.progress_bar(self.rect(), dir, self.value);
        }
    }
}
