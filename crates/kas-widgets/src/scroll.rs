// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use kas::event::{components::ScrollComponent, Scroll};
use kas::prelude::*;
use std::fmt::Debug;

impl_scope! {
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
    /// [`ScrollBarRegion`]: crate::ScrollBarRegion
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Debug, Default)]
    #[widget {
        Data = W::Data;
    }]
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
            *cx |= self.scroll.set_offset(offset);
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

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            let child_size = (rect.size - self.frame_size).max(self.min_child_size);
            let child_rect = Rect::new(rect.pos + self.offset, child_size);
            self.inner.set_rect(cx, child_rect);
            let _ = self
                .scroll
                .set_sizes(rect.size, child_size + self.frame_size);
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            self.inner.find_id(coord + self.translation())
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.with_clip_region(self.core.rect, self.scroll_offset(), |mut draw| {
                draw.recurse(&mut self.inner);
            });
        }
    }

    impl Events for Self {
        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            self.scroll
                .scroll_by_event(cx, event, self.id(), self.core.rect)
                .1
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, scroll: Scroll) {
            self.scroll.scroll(cx, self.rect(), scroll);
        }
    }
}
