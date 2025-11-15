// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapter widgets (wrappers)

mod adapt;
mod adapt_cx;
mod adapt_events;
mod adapt_widget;
mod reserve;
mod with_label;

pub use adapt::{Adapt, Map};
pub use adapt_cx::{AdaptConfigCx, AdaptEventCx};
pub use adapt_events::AdaptEvents;
pub use adapt_widget::*;
#[doc(inline)] pub use kas::widgets::adapt::*;
pub use reserve::{Margins, Reserve};
pub use with_label::{WithHiddenLabel, WithLabel};

#[allow(unused)] use kas::event::{ConfigCx, EventCx};
use kas::layout::{AxisInfo, SizeRules, Stretch};
use kas::theme::SizeCx;
use kas::{Layout, Widget, impl_self};

#[impl_self]
mod WithStretch {
    /// Adjust stretch rules
    ///
    /// Usually, this type will be constructed through one of the methods on
    /// trait [`AdaptWidget`].
    #[derive_widget]
    pub struct WithStretch<W: Widget> {
        #[widget]
        pub inner: W,
        /// Horizontal stretch
        ///
        /// Use [`ConfigCx::resize`] to apply changes.
        pub horiz: Option<Stretch>,
        /// Vertical stretch
        ///
        /// Use [`ConfigCx::resize`] to apply changes.
        pub vert: Option<Stretch>,
    }

    impl Self {
        /// Construct
        pub fn new(
            inner: W,
            horiz: impl Into<Option<Stretch>>,
            vert: impl Into<Option<Stretch>>,
        ) -> Self {
            WithStretch {
                inner,
                horiz: horiz.into(),
                vert: vert.into(),
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = self.inner.size_rules(cx, axis);
            if axis.is_horizontal()
                && let Some(stretch) = self.horiz
            {
                rules.set_stretch(stretch);
            } else if axis.is_vertical()
                && let Some(stretch) = self.vert
            {
                rules.set_stretch(stretch);
            }
            rules
        }
    }
}
