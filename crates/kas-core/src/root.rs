// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use crate::dir::Directional;
use crate::event::{ConfigMgr, EventMgr};
use crate::geom::{Coord, Rect, Size};
use crate::layout::{self, AxisInfo, SizeRules};
use crate::theme::{DrawMgr, SizeMgr};
use crate::{Layout, TkAction, Widget, WidgetExt, WidgetId, Window, WindowId};
use kas_macros::{autoimpl, impl_scope};
use smallvec::SmallVec;

impl_scope! {
    /// A support layer around a window
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[autoimpl(Deref, DerefMut using self.w)]
    #[derive(Debug)]
    #[widget]
    pub struct RootWidget {
        core: widget_core!(),
        #[widget]
        w: Box<dyn Window>,
        popups: SmallVec<[(WindowId, kas::Popup); 16]>,
    }

    impl Layout for RootWidget {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.w.size_rules(size_mgr, axis)
        }

        #[inline]
        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;
            self.w.set_rect(mgr, rect);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.core.rect.contains(coord) {
                return None;
            }
            for popup in self.popups.iter_mut().rev() {
                if let Some(id) = self
                    .w
                    .find_widget_mut(&popup.1.id)
                    .and_then(|w| w.find_id(coord))
                {
                    return Some(id);
                }
            }
            self.w.find_id(coord).or_else(|| Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.recurse(&mut self.w);
            for (_, popup) in &self.popups {
                if let Some(widget) = self.w.find_widget_mut(&popup.id) {
                    draw.with_overlay(widget.rect(), |mut draw| {
                        draw.recurse(widget);
                    });
                }
            }
        }
    }
}

impl RootWidget {
    /// Construct
    pub fn new(w: Box<dyn Window>) -> RootWidget {
        RootWidget {
            core: Default::default(),
            w,
            popups: Default::default(),
        }
    }

    /// Add a pop-up as a layer in the current window
    ///
    /// Each [`crate::Popup`] is assigned a [`WindowId`]; both are passed.
    pub fn add_popup(&mut self, mgr: &mut EventMgr, id: WindowId, popup: kas::Popup) {
        let index = self.popups.len();
        self.popups.push((id, popup));
        mgr.config_mgr(|mgr| self.resize_popup(mgr, index));
        mgr.send_action(TkAction::REDRAW);
    }

    /// Trigger closure of a pop-up
    ///
    /// If the given `id` refers to a pop-up, it should be closed.
    pub fn remove_popup(&mut self, mgr: &mut EventMgr, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                mgr.send_action(TkAction::REGION_MOVED);
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

// This is like WidgetChildren::find, but returns a translated Rect.
fn find_rect(widget: &dyn Widget, id: WidgetId) -> Option<Rect> {
    match widget.find_child_index(&id) {
        Some(i) => {
            if let Some(w) = widget.get_child(i) {
                find_rect(w, id).map(|rect| rect - widget.translation())
            } else {
                None
            }
        }
        None if widget.eq_id(&id) => Some(widget.rect()),
        _ => None,
    }
}

impl RootWidget {
    fn resize_popup(&mut self, mgr: &mut ConfigMgr, index: usize) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.core.rect;
        let popup = &mut self.popups[index].1;

        let c = find_rect(&self.w, popup.parent.clone()).unwrap();
        let widget = self.w.find_widget_mut(&popup.id).unwrap();
        let mut cache = layout::SolveCache::find_constraints(widget, mgr.size_mgr());
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

        cache.apply_rect(widget, mgr, rect, false, true);
    }
}
