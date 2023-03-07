// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use crate::dir::Directional;
use crate::event::{ConfigMgr, EventMgr, Scroll};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::layout::{self, AxisInfo, SizeRules};
use crate::theme::{DrawMgr, FrameStyle, SizeMgr};
use crate::title_bar::TitleBar;
use crate::{Action, Decorations, Layout, Node, Widget, WidgetExt, WidgetId, Window, WindowId};
use kas_macros::impl_scope;
use smallvec::SmallVec;

impl_scope! {
    /// A support layer around a window
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[derive(Debug)]
    #[widget]
    pub struct RootWidget {
        core: widget_core!(),
        decorations: Decorations,
        #[widget]
        title_bar: TitleBar,
        #[widget]
        w: Box<dyn Window>,
        data: (),
        bar_h: i32,
        dec_offset: Offset,
        dec_size: Size,
        popups: SmallVec<[(WindowId, kas::Popup, Offset); 16]>,
    }

    impl Layout for RootWidget {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let mut inner = self.w.size_rules(size_mgr.re(), axis);

            self.bar_h = 0;
            if matches!(self.decorations, Decorations::Toolkit) {
                let bar = self.title_bar.size_rules(size_mgr.re(), axis);
                if axis.is_horizontal() {
                    inner.max_with(bar);
                } else {
                    inner.append(bar);
                    self.bar_h = bar.min_size();
                }
            }
            if matches!(self.decorations, Decorations::Border | Decorations::Toolkit) {
                let frame = size_mgr.frame(FrameStyle::Window, axis);
                let (rules, offset, size) = frame.surround(inner);
                self.dec_offset.set_component(axis, offset);
                self.dec_size.set_component(axis, size);
                rules
            } else {
                inner
            }
        }

        #[inline]
        fn set_rect(&mut self, mgr: &mut ConfigMgr, mut rect: Rect) {
            self.core.rect = rect;
            rect.pos += self.dec_offset;
            rect.size -= self.dec_size;
            if self.bar_h > 0 {
                let bar_size = Size(rect.size.0, self.bar_h);
                self.title_bar.set_rect(mgr, Rect::new(rect.pos, bar_size));
                rect.pos.1 += self.bar_h;
                rect.size -= Size(0, self.bar_h);
            }
            self.w.set_rect(mgr, rect);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.core.rect.contains(coord) {
                return None;
            }
            for (_, popup, translation) in self.popups.iter_mut().rev() {
                if let Some(id) = self
                    .w
                    .find_widget(&self.data, &popup.id)
                    .and_then(|mut w| w.find_id(coord + *translation))
                {
                    return Some(id);
                }
            }
            self.title_bar.find_id(coord)
                .or_else(|| self.w.find_id(coord))
                .or_else(|| Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            if self.dec_size != Size::ZERO {
                draw.frame(self.core.rect, FrameStyle::Window, Default::default());
                if self.bar_h > 0 {
                    draw.recurse(&mut self.title_bar);
                }
            }
            draw.recurse(&mut self.w);
            for (_, popup, translation) in &self.popups {
                if let Some(mut widget) = self.w.find_widget(&self.data, &popup.id) {
                    let clip_rect = widget.rect() - *translation;
                    draw.with_overlay(clip_rect, *translation, |mut draw| {
                        widget.draw(draw.re_id(widget.id()));
                    });
                }
            }
        }
    }

    impl Widget for RootWidget {
        fn configure(&mut self, mgr: &mut ConfigMgr) {
            self.decorations = self.w.decorations();
            if mgr.platform().is_wayland() && self.decorations == Decorations::Server {
                // Wayland's base protocol does not support server-side decorations
                // TODO: Wayland has extensions for this; server-side is still
                // usually preferred where supported (e.g. KDE).
                self.decorations = Decorations::Toolkit;
            }
        }

        fn handle_scroll(&mut self, mgr: &mut EventMgr, _: Scroll) {
            // Something was scrolled; update pop-up translations
            mgr.config_mgr(|mgr| self.resize_popups(mgr));
        }
    }

    // Note: we do not simply Deref to self.w; that allows skipping our Layout
    // and Widget methods.
    impl Window for Self {
        #[inline]
        fn title(&self) -> &str {
            self.w.title()
        }

        #[inline]
        fn icon(&self) -> Option<crate::Icon> {
            self.w.icon()
        }

        #[inline]
        fn decorations(&self) -> crate::Decorations {
            self.w.decorations()
        }

        #[inline]
        fn restrict_dimensions(&self) -> (bool, bool) {
            self.w.restrict_dimensions()
        }

        #[inline]
        fn drag_anywhere(&self) -> bool {
            self.w.drag_anywhere()
        }

        #[inline]
        fn transparent(&self) -> bool {
            self.w.transparent()
        }

        #[inline]
        fn handle_closure(&mut self, mgr: &mut EventMgr) {
            self.w.handle_closure(mgr);
        }
    }
}

impl RootWidget {
    /// Construct
    pub fn new(w: Box<dyn Window>) -> RootWidget {
        RootWidget {
            core: Default::default(),
            decorations: Decorations::None,
            title_bar: TitleBar::new(w.title().to_string()),
            w,
            data: (),
            bar_h: 0,
            dec_offset: Default::default(),
            dec_size: Default::default(),
            popups: Default::default(),
        }
    }

    /// Access as a [`Node`]
    pub fn as_node(&mut self) -> Node {
        self.w.as_node(&self.data)
    }

    /// Add a pop-up as a layer in the current window
    ///
    /// Each [`crate::Popup`] is assigned a [`WindowId`]; both are passed.
    pub fn add_popup(&mut self, mgr: &mut EventMgr, id: WindowId, popup: kas::Popup) {
        let index = self.popups.len();
        self.popups.push((id, popup, Offset::ZERO));
        mgr.config_mgr(|mgr| self.resize_popup(mgr, index));
        mgr.send_action(Action::REDRAW);
    }

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    pub fn remove_popup(&mut self, mgr: &mut EventMgr, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                mgr.send_action(Action::REGION_MOVED);
                return;
            }
        }
    }

    /// Resize popups
    ///
    /// This is called immediately after [`Layout::set_rect`] to resize
    /// existing pop-ups.
    pub fn resize_popups(&mut self, mgr: &mut ConfigMgr) {
        for i in 0..self.popups.len() {
            self.resize_popup(mgr, i);
        }
    }
}

// Search for a widget by `id`. On success, return that widget's [`Rect`] and
// the translation of its children.
fn find_rect(widget: Node, id: WidgetId) -> Option<(Rect, Offset)> {
    fn inner(mut w: Node, id: WidgetId, t: Offset) -> Option<(Rect, Offset)> {
        if let Some(i) = w.find_child_index(&id) {
            let t = t + w.translation();
            if let Some(w2) = w.re().get_child(i) {
                return inner(w2, id, t);
            }
        }

        return if w.eq_id(&id) {
            if w.translation() != Offset::ZERO {
                // Unvalidated: does this cause issues with the parent's event handlers?
                log::warn!("Parent of pop-up {} has non-zero translation", w.identify());
            }

            Some((w.rect(), t))
        } else {
            None
        };
    }
    inner(widget, id, Offset::ZERO)
}

impl RootWidget {
    fn resize_popup(&mut self, mgr: &mut ConfigMgr, index: usize) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.core.rect;
        let (_, ref mut popup, ref mut translation) = self.popups[index];
        let data = &self.data;

        let (c, t) = find_rect(self.w.as_node(data), popup.parent.clone()).unwrap();
        *translation = t;
        let r = r + t; // work in translated coordinate space
        let mut widget = self.w.find_widget(data, &popup.id).unwrap();
        let mut cache = layout::SolveCache::find_constraints(widget.re(), mgr.size_mgr());
        let ideal = cache.ideal(false);
        let m = cache.margins();

        let is_reversed = popup.direction.is_reversed();
        let place_in = |rp, rs: i32, cp: i32, cs: i32, ideal, m: (u16, u16)| -> (i32, i32) {
            let m: (i32, i32) = (m.0.into(), m.1.into());
            let before: i32 = cp - (rp + m.1);
            let before = before.max(0);
            let after = (rs - (cs + before + m.0)).max(0);
            if after >= ideal {
                if is_reversed && before >= ideal {
                    (cp - ideal - m.1, ideal)
                } else {
                    (cp + cs + m.0, ideal)
                }
            } else if before >= ideal {
                (cp - ideal - m.1, ideal)
            } else if before > after {
                (rp, before)
            } else {
                (cp + cs + m.0, after)
            }
        };
        #[allow(clippy::manual_clamp)]
        let place_out = |rp, rs, cp: i32, cs, ideal: i32| -> (i32, i32) {
            let pos = cp.min(rp + rs - ideal).max(rp);
            let size = ideal.max(cs).min(rs);
            (pos, size)
        };
        let rect = if popup.direction.is_horizontal() {
            let (x, w) = place_in(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0, m.horiz);
            let (y, h) = place_out(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1);
            Rect::new(Coord(x, y), Size::new(w, h))
        } else {
            let (x, w) = place_out(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0);
            let (y, h) = place_in(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1, m.vert);
            Rect::new(Coord(x, y), Size::new(w, h))
        };

        cache.apply_rect(widget.re(), mgr, rect, false);
        cache.print_widget_heirarchy(widget);
    }
}
