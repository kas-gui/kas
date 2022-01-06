// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Progress bar

use std::fmt::Debug;

use kas::prelude::*;

widget! {
    /// A progress bar
    ///
    /// The "progress" value may range from 0.0 to 1.0.
    #[derive(Clone, Debug, Default)]
    pub struct ProgressBar<D: Directional> {
        #[widget_core]
        core: CoreData,
        direction: D,
        width: i32,
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
                direction,
                width: 0,
                value: 0.0,
            }
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
            let mut size = size_mgr.progress_bar();
            if self.direction.is_vertical() {
                size = size.transpose();
            }
            let margins = (0, 0);
            if self.direction.is_vertical() == axis.is_vertical() {
                SizeRules::new(size.0, size.0, margins, Stretch::High)
            } else {
                self.width = size.1;
                SizeRules::fixed(size.1, margins)
            }
        }

        fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
            let mut ideal_size = Size::splat(self.width);
            ideal_size.set_component(self.direction, i32::MAX);
            let rect = align
                .complete(Align::Center, Align::Center)
                .aligned_rect(ideal_size, rect);
            self.core.rect = rect;
        }

        fn draw(&mut self, mut draw: DrawMgr, mgr: &ManagerState, disabled: bool) {
            let dir = self.direction.as_direction();
            let state = self.input_state(mgr, disabled);
            draw.progress_bar(self.core.rect, dir, state, self.value);
        }
    }
}
