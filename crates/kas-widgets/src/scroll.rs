// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use kas::event::{CursorIcon, Scroll, components::ScrollComponent};
use kas::prelude::*;
use std::fmt::Debug;

#[impl_self]
mod ScrollRegion {
    /// A scrollable region
    ///
    /// This region supports scrolling via mouse wheel and click/touch drag.
    ///
    /// The ideal size of a `ScrollRegion` is the ideal size of its content:
    /// that is, all content may be shown at ideal size without scrolling.
    /// The minimum size of a `ScrollRegion` is somewhat arbitrary (currently,
    /// fixed at the height of three lines of standard text). The inner size
    /// (content size) is `max(content_min_size, outer_size - content_margin)`.
    ///
    /// Scroll bars are not included; use [`ScrollBarRegion`] if you want those.
    ///
    /// ### Messages
    ///
    /// [`kas::messages::SetScrollOffset`] may be used to set the scroll offset.
    ///
    /// [`ScrollBarRegion`]: crate::ScrollBarRegion
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct ScrollRegion<W: Widget> {
        core: widget_core!(),
        min_child_size: Size,
        offset: Offset,
        frame_size: Size,
        scroll: ScrollComponent,
        #[widget]
        inner: W,
    }

    impl Self {
        /// Construct a new scroll region around an inner widget
        #[inline]
        pub fn new(inner: W) -> Self {
            ScrollRegion {
                core: Default::default(),
                min_child_size: Size::ZERO,
                offset: Default::default(),
                frame_size: Default::default(),
                scroll: Default::default(),
                inner,
            }
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner(&self) -> &W {
            &self.inner
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner_mut(&mut self) -> &mut W {
            &mut self.inner
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            (
                self.min_child_size.0 > size.0,
                self.min_child_size.1 > size.1,
            )
        }

        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.scroll.max_offset()
        }

        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.scroll.offset()
        }

        #[inline]
        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let action = self.scroll.set_offset(offset);
            cx.action(&self, action);
            self.scroll.offset()
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.sub_other(self.frame_size.extract(axis.flipped()));

            let mut rules = self.inner.size_rules(sizer.re(), axis);
            self.min_child_size.set_component(axis, rules.min_size());
            rules.reduce_min_to(sizer.min_scroll_size(axis));

            // We use a frame to contain the content margin within the scrollable area.
            let frame = kas::layout::FrameRules::ZERO;
            let (rules, offset, size) = frame.surround(rules);
            self.offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            let child_size = (rect.size - self.frame_size).max(self.min_child_size);
            let child_rect = Rect::new(rect.pos, child_size);
            self.inner.set_rect(cx, child_rect, hints);
            let _ = self
                .scroll
                .set_sizes(rect.size, child_size + self.frame_size);
        }

        fn draw(&self, mut draw: DrawCx) {
            draw.with_clip_region(self.rect(), self.scroll_offset(), |mut draw| {
                self.inner.draw(draw.re());
            });
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollRegion {
                offset: self.scroll_offset(),
                max_offset: self.max_scroll_offset(),
            }
        }

        #[inline]
        fn translation(&self, _: usize) -> Offset {
            self.scroll_offset()
        }

        fn probe(&self, coord: Coord) -> Id {
            if self.scroll.is_kinetic_scrolling() {
                return self.id();
            }

            self.inner
                .try_probe(coord + self.scroll_offset())
                .unwrap_or_else(|| self.id())
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            self.scroll
                .is_kinetic_scrolling()
                .then_some(CursorIcon::AllScroll)
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            self.scroll
                .scroll_by_event(cx, event, self.id(), self.rect())
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                self.set_scroll_offset(cx, offset);
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, scroll: Scroll) {
            self.scroll.scroll(cx, self.id(), self.rect(), scroll);
        }
    }
}
