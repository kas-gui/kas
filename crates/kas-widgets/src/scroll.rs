// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use super::Scrollable;
use kas::event::{self, components::ScrollComponent};
use kas::prelude::*;
use kas::theme::TextClass;
use std::fmt::Debug;

widget! {
    /// A scrollable region
    ///
    /// This region supports scrolling via mouse wheel and click/touch drag.
    ///
    /// Scrollbars are not included; use [`ScrollBarRegion`] if you want those.
    ///
    /// [`ScrollBarRegion`]: crate::ScrollBarRegion
    #[autoimpl(Deref, DerefMut on self.inner)]
    #[autoimpl(class_traits where W: trait on self.inner)]
    #[derive(Clone, Debug, Default)]
    #[handler(msg = <W as event::Handler>::Msg)]
    pub struct ScrollRegion<W: Widget> {
        #[widget_core]
        core: CoreData,
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
        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            *mgr |= self.scroll.set_offset(offset);
            self.scroll.offset()
        }
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut EventMgr, id: WidgetId) {
            self.core_data_mut().id = id;
            mgr.register_nav_fallback(self.id());
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let mut rules = self.inner.size_rules(size_mgr.re(), axis);
            self.min_child_size.set_component(axis, rules.min_size());
            let line_height = size_mgr.line_height(TextClass::Label);
            self.scroll.set_scroll_rate(3.0 * f32::conv(line_height));
            rules.reduce_min_to(line_height);

            // We use a zero-sized frame to push any margins inside the scroll-region.
            let frame = kas::layout::FrameRules::new(0, 0, 0, (0, 0));
            let (rules, offset, size) = frame.surround_with_margin(rules);
            self.offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let child_size = (rect.size - self.frame_size).max(self.min_child_size);
            let child_rect = Rect::new(rect.pos + self.offset, child_size);
            self.inner.set_rect(mgr, child_rect, align);
            let _ = self
                .scroll
                .set_sizes(rect.size, child_size + self.frame_size);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            self.inner.find_id(coord + self.translation())
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let mut draw = draw.with_core(self.core_data());
            draw.with_clip_region(self.core.rect, self.scroll_offset(), |mut draw| {
                self.inner.draw(draw.re())
            });
        }
    }

    impl event::SendEvent for Self {
        fn send(&mut self, mgr: &mut EventMgr, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.is_disabled() {
                return Response::Unused;
            }

            if self.inner.id().is_ancestor_of(&id) {
                let child_event = self.scroll.offset_event(event.clone());
                match self.inner.send(mgr, id, child_event) {
                    Response::Unused => (),
                    Response::Pan(delta) => {
                        return match self.scroll_by_delta(mgr, delta) {
                            delta if delta == Offset::ZERO => Response::Scrolled,
                            delta => Response::Pan(delta),
                        };
                    }
                    Response::Focus(rect) => {
                        let (rect, action) = self.scroll.focus_rect(rect, self.core.rect);
                        *mgr |= action;
                        return Response::Focus(rect);
                    }
                    r => return r,
                }
            } else {
                debug_assert!(self.eq_id(id), "SendEvent::send: bad WidgetId");
            };

            let id = self.id();
            let (action, response) =
                self.scroll
                    .scroll_by_event(mgr, event, self.id(), self.core.rect.size, |mgr, source, _, coord| {
                        if source.is_primary() && mgr.config_enable_mouse_pan() {
                            let icon = Some(event::CursorIcon::Grabbing);
                            mgr.grab_press_unique(id, source, coord, icon);
                        }
                    });
            if !action.is_empty() {
                *mgr |= action;
                Response::Focus(self.core.rect)
            } else {
                response.void_into()
            }
        }
    }
}
