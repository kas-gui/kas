// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hidden extras
//!
//! It turns out that some widgets are needed in kas-core. This module is
//! hidden by default and direct usage (outside of kas crates) is
//! not supported (i.e. **changes are not considered breaking**).

#[allow(unused)] use crate::Action;
use crate::event::ConfigCx;
use crate::geom::Rect;
use crate::layout::AlignHints;
use crate::theme::{Text, TextClass};
use crate::{Events, Layout, Role, RoleCx, Tile};
use kas_macros::impl_self;

#[impl_self]
mod StrLabel {
    /// A simple text label with static contents
    ///
    /// Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled.
    #[derive(Clone, Debug, Default)]
    #[widget]
    #[layout(self.text)]
    pub struct StrLabel {
        core: widget_core!(),
        text: Text<&'static str>,
    }

    impl Self {
        /// Construct from `text`
        #[inline]
        pub fn new(text: &'static str) -> Self {
            StrLabel {
                core: Default::default(),
                text: Text::new(text, TextClass::Label(false)),
            }
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl Layout for Self {
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.text
                .set_rect(cx, rect, hints.combine(AlignHints::VERT_CENTER));
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Label(self.text.as_str())
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }
    }
}
