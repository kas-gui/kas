// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use crate::{ScrollBar, ScrollBarMode, ScrollBarMsg};
use kas::event::{CursorIcon, Scroll, components::ScrollComponent};
use kas::prelude::*;
use std::fmt::Debug;

#[impl_self]
mod ClipRegion {
    /// A region which clips its contents to a [`Viewport`]
    ///
    /// This is a low-level widget supporting content larger on the inside, but
    /// without handling scrolling. You probably want to use [`ScrollRegion`]
    /// instead.
    ///
    /// ### Size
    ///
    /// Kas's size model allows widgets to advertise two sizes: the *minimum*
    /// size and the *ideal* size. This distinction is used to full effect here:
    ///
    /// -   The ideal size is that of the inner content, thus avoiding any need
    ///     to scroll content.
    /// -   The minimum size is an arbitrary size defined by the theme
    ///     ([`SizeCx::min_scroll_size`]).
    #[derive(Debug, Default)]
    #[widget]
    pub struct ClipRegion<W: Widget> {
        core: widget_core!(),
        min_child_size: Size,
        offset: Offset,
        frame_size: Size,
        #[widget]
        inner: W,
    }

    impl Self {
        /// Construct a new scroll region around an inner widget
        #[inline]
        pub fn new(inner: W) -> Self {
            ClipRegion {
                core: Default::default(),
                min_child_size: Size::ZERO,
                offset: Default::default(),
                frame_size: Default::default(),
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

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, mut axis: AxisInfo) -> SizeRules {
            let dir = axis.as_direction();
            axis.map_other(|x| {
                (x - self.frame_size.extract(dir)).max(self.min_child_size.extract(dir))
            });

            let mut rules = self.inner.size_rules(cx, axis);
            self.min_child_size.set_component(axis, rules.min_size());
            rules.reduce_min_to(cx.min_scroll_size(axis));

            // We use a frame to contain the content margin within the scrollable area.
            let frame = kas::layout::FrameRules::ZERO;
            let (rules, offset, size) = frame.surround(rules);
            self.offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            self.core.set_rect(rect);
            let child_size = (rect.size - self.frame_size).max(self.min_child_size);
            let child_rect = Rect::new(rect.pos, child_size);
            self.inner.set_rect(cx, child_rect, hints);
        }
    }

    impl Viewport for Self {
        #[inline]
        fn content_size(&self) -> Size {
            self.min_child_size
        }

        fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            // We use a new pass to clip and offset scrolled content:
            draw.with_clip_region(rect, offset, |mut draw| {
                self.inner.draw(draw.re());
            });
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn probe(&self, coord: Coord) -> Id {
            self.inner.try_probe(coord).unwrap_or_else(|| self.id())
        }
    }
}

#[impl_self]
mod ScrollRegion {
    /// A region which supports scrolling of content through a viewport
    ///
    /// This region supports scrolling via mouse wheel and click/touch drag
    /// as well as using scroll bars (optional).
    ///
    /// ### Size
    ///
    /// Kas's size model allows widgets to advertise two sizes: the *minimum*
    /// size and the *ideal* size. This distinction is used to full effect here:
    ///
    /// -   The ideal size is that of the inner content, thus avoiding any need
    ///     to scroll content.
    /// -   The minimum size is an arbitrary size defined by the theme
    ///     ([`SizeCx::min_scroll_size`]).
    ///
    /// ### Generic usage
    ///
    /// Though this widget is generic over any [`Viewport`], it is primarily
    /// intended for usage with [`ClipRegion`]; the primary constructor
    /// [`Self::new_clip`] uses this while [`Self::new_viewport`] allows usage
    /// with other implementations of [`Viewport`].
    ///
    /// It should be noted that scroll bar positioning does not respect the
    /// inner widget's margins, since the result looks poor when content is
    /// scrolled. Instead the inner widget should force internal margins by
    /// wrapping contents with a (zero-sized) frame.
    ///
    /// ### Messages
    ///
    /// [`kas::messages::SetScrollOffset`] may be used to set the scroll offset.
    #[derive(Debug, Default)]
    #[widget]
    pub struct ScrollRegion<W: Viewport + Widget> {
        core: widget_core!(),
        scroll: ScrollComponent,
        mode: ScrollBarMode,
        show_bars: (bool, bool), // set by user (or set_rect when mode == Auto)
        hints: AlignHints,
        #[widget(&())]
        horiz_bar: ScrollBar<kas::dir::Right>,
        #[widget(&())]
        vert_bar: ScrollBar<kas::dir::Down>,
        #[widget]
        inner: W,
    }

    impl<Inner: Widget> ScrollRegion<ClipRegion<Inner>> {
        /// Construct a scroll region using a [`ClipRegion`]
        ///
        /// This is probably the constructor you want *unless* the inner widget
        /// already implements [`Viewport`].
        ///
        /// Uses [`ScrollBarMode::Auto`] by default.
        #[inline]
        pub fn new_clip(inner: Inner) -> Self {
            Self::new_viewport(ClipRegion::new(inner))
        }
    }

    impl Self {
        /// Construct over a [`Viewport`]
        ///
        /// Uses [`ScrollBarMode::Auto`] by default.
        #[inline]
        pub fn new_viewport(inner: W) -> Self {
            ScrollRegion {
                core: Default::default(),
                scroll: Default::default(),
                mode: ScrollBarMode::Auto,
                show_bars: (false, false),
                hints: Default::default(),
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
            self.mode = ScrollBarMode::Fixed(horiz, vert);
            self.horiz_bar.set_invisible(false);
            self.vert_bar.set_invisible(false);
            self.show_bars = (horiz, vert);
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
            self.mode = ScrollBarMode::Invisible(horiz, vert);
            self.horiz_bar.set_invisible(true);
            self.vert_bar.set_invisible(true);
            self.show_bars = (horiz, vert);
            self
        }

        /// Get current mode of scroll bars
        #[inline]
        pub fn scroll_bar_mode(&self) -> ScrollBarMode {
            self.mode
        }

        /// Set scroll bar mode
        pub fn set_scroll_bar_mode(&mut self, cx: &mut ConfigCx, mode: ScrollBarMode) {
            if mode != self.mode {
                self.mode = mode;
                let (invis_horiz, invis_vert) = match mode {
                    ScrollBarMode::Auto => (false, false),
                    ScrollBarMode::Fixed(horiz, vert) => {
                        self.show_bars = (horiz, vert);
                        (false, false)
                    }
                    ScrollBarMode::Invisible(horiz, vert) => {
                        self.show_bars = (horiz, vert);
                        (horiz, vert)
                    }
                };
                self.horiz_bar.set_invisible(invis_horiz);
                self.vert_bar.set_invisible(invis_vert);
                cx.resize();
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

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = self.inner.size_rules(cx, axis);
            let vert_rules = self.vert_bar.size_rules(cx, axis);
            let horiz_rules = self.horiz_bar.size_rules(cx, axis);
            let (use_horiz, use_vert) = match self.mode {
                ScrollBarMode::Fixed(horiz, vert) => (horiz, vert),
                ScrollBarMode::Auto => (true, true),
                ScrollBarMode::Invisible(_, _) => (false, false),
            };
            if axis.is_horizontal() && use_horiz {
                rules.append(vert_rules);
            } else if axis.is_vertical() && use_vert {
                rules.append(horiz_rules);
            }
            rules
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            self.core.set_rect(rect);
            self.hints = hints;
            let pos = rect.pos;
            let mut child_size = rect.size;

            let bar_width = cx.scroll_bar_width();
            let content_size = self.inner.content_size();
            if self.mode == ScrollBarMode::Auto {
                let max_offset = content_size - child_size;
                self.show_bars.0 = max_offset.0 > 0;
                self.show_bars.1 = max_offset.1 > 0;
            }
            if self.show_bars.0 && !self.horiz_bar.is_invisible() {
                child_size.1 -= bar_width;
            }
            if self.show_bars.1 && !self.vert_bar.is_invisible() {
                child_size.0 -= bar_width;
            }

            let child_rect = Rect::new(pos, child_size);
            self.inner.set_rect(cx, child_rect, hints);

            let _ = self.scroll.set_sizes(child_size, content_size);
            let offset = self.scroll.offset();
            let max_scroll_offset = self.scroll.max_offset();
            self.inner.set_offset(cx, child_rect, offset);

            if self.show_bars.0 {
                let pos = Coord(pos.0, rect.pos2().1 - bar_width);
                let size = Size::new(child_size.0, bar_width);
                self.horiz_bar
                    .set_rect(cx, Rect { pos, size }, AlignHints::NONE);
                self.horiz_bar
                    .set_limits(cx, max_scroll_offset.0, rect.size.0);
                self.horiz_bar.set_value(cx, offset.0);
            } else {
                self.horiz_bar.set_rect(cx, Rect::ZERO, AlignHints::NONE);
            }

            if self.show_bars.1 {
                let pos = Coord(rect.pos2().0 - bar_width, pos.1);
                let size = Size::new(bar_width, self.rect().size.1);
                self.vert_bar
                    .set_rect(cx, Rect { pos, size }, AlignHints::NONE);
                self.vert_bar
                    .set_limits(cx, max_scroll_offset.1, rect.size.1);
                self.vert_bar.set_value(cx, offset.1);
            } else {
                self.vert_bar.set_rect(cx, Rect::ZERO, AlignHints::NONE);
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let viewport = self.inner.rect();
            self.inner
                .draw_with_offset(draw.re(), viewport, self.scroll.offset());
            if self.show_bars == (false, false) {
                return;
            }

            // We use a new pass to draw scroll bars over inner content, but
            // only when required to minimize cost:
            let ev_state = draw.ev_state();
            if matches!(self.mode, ScrollBarMode::Invisible(_, _))
                && (self.horiz_bar.currently_visible(ev_state)
                    || self.vert_bar.currently_visible(ev_state))
            {
                draw.with_pass(|mut draw| {
                    if self.show_bars.0 {
                        self.horiz_bar.draw(draw.re());
                    }
                    if self.show_bars.1 {
                        self.vert_bar.draw(draw.re());
                    }
                });
            } else {
                if self.show_bars.0 {
                    self.horiz_bar.draw(draw.re());
                }
                if self.show_bars.1 {
                    self.vert_bar.draw(draw.re());
                }
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollRegion {
                offset: self.scroll.offset(),
                max_offset: self.scroll.max_offset(),
            }
        }

        #[inline]
        fn translation(&self, index: usize) -> Offset {
            if index == widget_index![self.inner] {
                self.scroll.offset()
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
            if self.scroll.is_kinetic_scrolling() {
                return self.id();
            }

            self.inner
                .try_probe_with_offset(coord, self.scroll.offset())
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

        fn handle_event(&mut self, cx: &mut EventCx, data: &Self::Data, event: Event) -> IsUsed {
            let initial_offset = self.scroll.offset();
            let is_used = self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.inner.rect());

            let offset = self.scroll.offset();
            if offset != initial_offset {
                self.horiz_bar.set_value(cx, offset.0);
                self.vert_bar.set_value(cx, offset.1);
                self.inner
                    .update_offset(cx, data, self.inner.rect(), offset);
            }

            is_used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Self::Data) {
            let index = cx.last_child();
            let offset = if index == Some(widget_index![self.horiz_bar])
                && let Some(ScrollBarMsg(x)) = cx.try_pop()
            {
                Offset(x, self.scroll.offset().1)
            } else if index == Some(widget_index![self.vert_bar])
                && let Some(ScrollBarMsg(y)) = cx.try_pop()
            {
                Offset(self.scroll.offset().0, y)
            } else if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                self.horiz_bar.set_value(cx, offset.0);
                self.vert_bar.set_value(cx, offset.1);
                offset
            } else {
                return;
            };

            let action = self.scroll.set_offset(offset);
            cx.action_moved(action);
            self.inner
                .update_offset(cx, data, self.inner.rect(), offset);
        }

        fn handle_resize(&mut self, cx: &mut ConfigCx, _: &Self::Data) -> ActionResize {
            let _ = self.size_rules(&mut cx.size_cx(), AxisInfo::new(false, None));
            let width = self.rect().size.0;
            let _ = self.size_rules(&mut cx.size_cx(), AxisInfo::new(true, Some(width)));
            self.set_rect(&mut cx.size_cx(), self.rect(), self.hints);
            ActionResize(false)
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &Self::Data, scroll: Scroll) {
            self.scroll.scroll(cx, self.id(), self.rect(), scroll);

            let offset = self.scroll.offset();
            self.horiz_bar.set_value(cx, offset.0);
            self.vert_bar.set_value(cx, offset.1);
            self.inner
                .update_offset(cx, data, self.inner.rect(), offset);
        }
    }
}
