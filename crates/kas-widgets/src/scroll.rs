// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use crate::{ScrollBar, ScrollBarMode, ScrollBarMsg, ScrollBarPair};
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
    /// ### Scroll bars
    ///
    /// Scroll bars are optional; by default they are auto-enabled as required.
    ///
    /// Scroll bar positioning does not respect the inner widget's margins, since
    /// the result looks poor when content is scrolled. Instead the content should
    /// force internal margins by wrapping contents with a (zero-sized) frame.
    /// [`ScrollRegion`] already does this.
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
        scroll_rect: Rect,
        min_child_size: Size,
        offset: Offset,
        frame_size: Size,
        hints: AlignHints,
        scroll: ScrollComponent,
        bars: ScrollBarPair,
        #[widget(&())]
        horiz_bar: ScrollBar<kas::dir::Right>,
        #[widget(&())]
        vert_bar: ScrollBar<kas::dir::Down>,
        #[widget]
        inner: W,
    }

    impl Self {
        /// Construct a new scroll region around an inner widget
        #[inline]
        pub fn new(inner: W) -> Self {
            ScrollRegion {
                core: Default::default(),
                scroll_rect: Default::default(),
                min_child_size: Size::ZERO,
                offset: Default::default(),
                frame_size: Default::default(),
                hints: Default::default(),
                scroll: Default::default(),
                bars: Default::default(),
                horiz_bar: ScrollBar::new(),
                vert_bar: ScrollBar::new(),
                inner,
            }
        }

        /// Set fixed visibility of scroll bars (inline)
        #[inline]
        pub fn with_fixed_bars(mut self, horiz: bool, vert: bool) -> Self
        where
            Self: Sized,
        {
            self.bars.set_mode(ScrollBarMode::Fixed(horiz, vert));
            self.horiz_bar.set_invisible(false);
            self.vert_bar.set_invisible(false);
            self
        }

        /// Set fixed, invisible bars (inline)
        ///
        /// In this mode scroll bars are either enabled but invisible until
        /// mouse over or disabled completely.
        #[inline]
        pub fn with_invisible_bars(mut self, horiz: bool, vert: bool) -> Self
        where
            Self: Sized,
        {
            self.bars.set_mode(ScrollBarMode::Invisible(horiz, vert));
            self.horiz_bar.set_invisible(true);
            self.vert_bar.set_invisible(true);
            self
        }

        /// Get current mode of scroll bars
        #[inline]
        pub fn bar_mode(&self) -> ScrollBarMode {
            self.bars.mode()
        }

        /// Set scroll bar mode
        pub fn set_bar_mode(&mut self, cx: &mut ConfigCx, mode: ScrollBarMode) {
            if mode != self.bars.mode() {
                self.bars.set_mode(mode);
                let (invis_horiz, invis_vert) = match mode {
                    ScrollBarMode::Auto => (false, false),
                    ScrollBarMode::Fixed(_, _) => (false, false),
                    ScrollBarMode::Invisible(horiz, vert) => (horiz, vert),
                };
                self.horiz_bar.set_invisible(invis_horiz);
                self.vert_bar.set_invisible(invis_vert);
                cx.resize();
            }
        }

        /// Set scroll bar mode (inline)
        ///
        /// This method should only be used during construction.
        #[inline]
        pub fn with_bar_mode(mut self, mode: ScrollBarMode) -> Self {
            self.bars.set_mode(mode);
            self
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
        #[inline]
        fn content_size(&self) -> Size {
            self.min_child_size
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
        fn set_scroll_offset(&mut self, cx: &mut EventState, offset: Offset) -> Offset {
            let action = self.scroll.set_offset(offset);
            cx.action_moved(action);
            let offset = self.scroll.offset();
            self.horiz_bar.set_value(cx, offset.0);
            self.vert_bar.set_value(cx, offset.1);
            offset
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, mut axis: AxisInfo) -> SizeRules {
            let dir = axis.as_direction();
            axis.map_other(|x| {
                (x - self.frame_size.extract(dir)).max(self.min_child_size.extract(dir))
            });

            let mut inner = self.inner.size_rules(cx, axis);
            // let m = inner.margins_i32();
            // let min = inner.min_size() + m.0 + m.1;
            self.min_child_size.set_component(axis, inner.min_size());
            inner.reduce_min_to(cx.min_scroll_size(axis));

            // We use a frame to contain the content margin within the scrollable area.
            let frame = kas::layout::FrameRules::ZERO;
            let (inner, offset, size) = frame.surround(inner);
            self.offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);

            self.bars
                .size_rules(cx, &mut self.horiz_bar, &mut self.vert_bar, inner, axis)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.hints = hints;
            let mut window_size = rect.size;

            let inner_size = window_size - self.frame_size;
            let need_to_scroll = (
                inner_size.0 < self.min_child_size.0,
                inner_size.1 < self.min_child_size.1,
            );

            window_size -=
                self.bars
                    .rect_size_reduction(cx, &self.horiz_bar, &self.vert_bar, need_to_scroll);

            self.scroll_rect = Rect::new(rect.pos, window_size);
            let child_size = (window_size - self.frame_size).max(self.min_child_size);
            let child_rect = Rect::new(rect.pos, child_size);
            self.inner.set_rect(cx, child_rect, hints);
            let _ = self
                .scroll
                .set_sizes(window_size, child_size + self.frame_size);

            self.bars.set_rects(
                cx,
                &mut self.horiz_bar,
                &mut self.vert_bar,
                rect,
                self.scroll.max_offset(),
            );
        }

        fn draw(&self, mut draw: DrawCx) {
            // We use a new pass to clip and offset scrolled content:
            draw.with_clip_region(self.scroll_rect, self.scroll_offset(), |mut draw| {
                self.inner.draw(draw.re());
            });

            self.bars.draw(draw, &self.horiz_bar, &self.vert_bar);
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
        fn translation(&self, index: usize) -> Offset {
            if index == widget_index![self.inner] {
                self.scroll_offset()
            } else {
                Offset::ZERO
            }
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn probe(&self, coord: Coord) -> Id {
            if let Some(id) = self
                .vert_bar
                .try_probe(coord)
                .or_else(|| self.horiz_bar.try_probe(coord))
            {
                return id;
            }

            (!self.scroll.is_kinetic_scrolling())
                .then(|| self.inner.try_probe(coord + self.scroll_offset()))
                .flatten()
                .unwrap_or_else(|| self.id())
        }

        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            self.scroll
                .is_kinetic_scrolling()
                .then_some(CursorIcon::AllScroll)
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            let is_used = self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.scroll_rect);

            let offset = self.scroll_offset();
            self.horiz_bar.set_value(cx, offset.0);
            self.vert_bar.set_value(cx, offset.1);
            is_used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            let index = cx.last_child();
            if index == Some(widget_index![self.horiz_bar])
                && let Some(ScrollBarMsg(x)) = cx.try_pop()
            {
                let offset = Offset(x, self.scroll_offset().1);
                let action = self.scroll.set_offset(offset);
                cx.action_moved(action);
            } else if index == Some(widget_index![self.vert_bar])
                && let Some(ScrollBarMsg(y)) = cx.try_pop()
            {
                let offset = Offset(self.scroll_offset().0, y);
                let action = self.scroll.set_offset(offset);
                cx.action_moved(action);
            } else if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                self.set_scroll_offset(cx, offset);
            }
        }

        fn handle_resize(&mut self, cx: &mut ConfigCx, _: &Self::Data) -> ActionResize {
            let _ = self.size_rules(&mut cx.size_cx(), AxisInfo::new(false, None));
            let width = self.scroll_rect.size.0;
            let _ = self.size_rules(&mut cx.size_cx(), AxisInfo::new(true, Some(width)));
            self.set_rect(&mut cx.size_cx(), self.rect(), self.hints);
            ActionResize(false)
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, scroll: Scroll) {
            self.scroll.scroll(cx, self.id(), self.scroll_rect, scroll);

            let offset = self.scroll_offset();
            self.horiz_bar.set_value(cx, offset.0);
            self.vert_bar.set_value(cx, offset.1);
        }
    }
}
