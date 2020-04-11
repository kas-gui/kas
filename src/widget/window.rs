// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use std::fmt::{self, Debug};

use kas::draw::{DrawHandle, SizeHandle};
use kas::event::{Callback, Event, Manager, Response, VoidMsg};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;
use kas::WindowId;

/// The main instantiation of the [`Window`] trait.
#[handler(action, generics = <> where W: Widget<Msg = VoidMsg>)]
#[widget(children=noauto, config=noauto)]
#[derive(Widget)]
pub struct Window<W: Widget + 'static> {
    #[widget_core]
    core: CoreData,
    restrict_dimensions: (bool, bool),
    title: CowString,
    #[widget]
    w: W,
    popups: Vec<(WindowId, kas::Popup)>,
    fns: Vec<(Callback, &'static dyn Fn(&mut W, &mut Manager))>,
}

impl<W: Widget> Debug for Window<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Window {{ core: {:?}, solver: <omitted>, w: {:?}, fns: [",
            self.core, self.w
        )?;
        let mut iter = self.fns.iter();
        if let Some(first) = iter.next() {
            write!(f, "({:?}, <Fn>)", first.0)?;
            for next in iter {
                write!(f, ", ({:?}, <Fn>)", next.0)?;
            }
        }
        write!(f, "] }}")
    }
}

impl<W: Widget + Clone> Clone for Window<W> {
    fn clone(&self) -> Self {
        Window {
            core: self.core.clone(),
            restrict_dimensions: self.restrict_dimensions.clone(),
            title: self.title.clone(),
            w: self.w.clone(),
            popups: vec![], // these are temporary; don't clone
            fns: self.fns.clone(),
        }
    }
}

impl<W: Widget> Window<W> {
    /// Create
    pub fn new<T: Into<CowString>>(title: T, w: W) -> Window<W> {
        Window {
            core: Default::default(),
            restrict_dimensions: (true, false),
            title: title.into(),
            w,
            popups: vec![],
            fns: Vec::new(),
        }
    }

    /// Configure whether min/max dimensions are forced
    ///
    /// By default, the min size is enforced but not the max.
    pub fn set_restrict_dimensions(&mut self, min: bool, max: bool) {
        self.restrict_dimensions = (min, max);
    }

    /// Add a closure to be called, with a reference to self, on the given
    /// condition. The closure must be passed by reference.
    // TODO: consider whether to keep this. The only functionality added is for
    // actions which happen on destruction.
    pub fn add_callback(&mut self, condition: Callback, f: &'static dyn Fn(&mut W, &mut Manager)) {
        self.fns.push((condition, f));
    }
}

impl<W: Widget> WidgetChildren for Window<W> {
    fn len(&self) -> usize {
        1 + self.popups.len()
    }

    fn get(&self, index: usize) -> Option<&dyn WidgetConfig> {
        if index == 0 {
            Some(&self.w)
        } else {
            self.popups
                .get(index - 1)
                .map(|popup| popup.1.overlay.as_widget())
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        if index == 0 {
            Some(&mut self.w)
        } else {
            self.popups
                .get_mut(index - 1)
                .map(|popup| popup.1.overlay.as_widget_mut())
        }
    }
}

impl<W: Widget> WidgetConfig for Window<W> {
    fn configure(&mut self, mgr: &mut Manager) {
        for (condition, f) in &self.fns {
            match condition {
                Callback::Start => f(&mut self.w, mgr),
                Callback::Close => (),
            }
        }
    }
}

impl<W: Widget> Layout for Window<W> {
    #[inline]
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        // Note: we do not consider popups, since they are usually temporary
        self.w.size_rules(size_handle, axis)
    }

    #[inline]
    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.w.set_rect(rect, align);
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if self.is_disabled() {
            return None;
        }

        for popup in self.popups.iter().rev() {
            if let Some(id) = popup.1.overlay.find_id(coord) {
                return Some(id);
            }
        }
        self.w.find_id(coord)
    }

    #[inline]
    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        self.w.draw(draw_handle, mgr);
        for popup in &self.popups {
            draw_handle.clip_region(self.core.rect, Coord::ZERO, &mut |draw_handle| {
                popup.1.overlay.draw(draw_handle, mgr);
            });
        }
    }
}

impl<W: Widget<Msg = VoidMsg> + 'static> event::EventHandler for Window<W> {
    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if id <= self.w.id() {
            self.w.event(mgr, id, event)
        } else {
            for i in 0..self.popups.len() {
                let widget = &mut self.popups[i].1.overlay;
                if id <= widget.id() {
                    let r = widget.event(mgr, id, event);
                    if mgr.replace_action_close_with_reconfigure() {
                        self.popups.remove(i);
                    }
                    return r;
                }
            }

            debug_assert!(id == self.id(), "EventHandler::event: bad WidgetId");
            Manager::handle_generic(self, mgr, event)
        }
    }
}

impl<W: Widget<Msg = VoidMsg> + 'static> kas::Window for Window<W> {
    fn title(&self) -> &str {
        &self.title
    }

    fn restrict_dimensions(&self) -> (bool, bool) {
        self.restrict_dimensions
    }

    fn add_popup(
        &mut self,
        _: &mut dyn SizeHandle,
        mgr: &mut Manager,
        id: WindowId,
        popup: kas::Popup,
    ) {
        // TODO: using reconfigure here is inefficient
        mgr.send_action(TkAction::Reconfigure);
        self.popups.push((id, popup));
    }

    fn remove_popup(&mut self, mgr: &mut Manager, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                mgr.send_action(TkAction::Reconfigure);
                return;
            }
        }
    }

    fn resize_popups(&mut self, size_handle: &mut dyn SizeHandle) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.core.rect;
        for (_id, popup) in &mut self.popups {
            let c = self.w.find(popup.parent).unwrap().rect();
            let widget = popup.overlay.as_widget_mut();
            let mut cache = layout::SolveCache::find_constraints(widget, size_handle);
            let ideal = cache.ideal(false);
            let m = cache.margins();

            let is_reversed = popup.direction.is_reversed();
            let place_in = |rp, rs: u32, cp: i32, cs, ideal, m: (u16, u16)| -> (i32, u32) {
                let before = (cp.saturating_sub(rp + m.1 as i32)) as u32;
                let after = rs.saturating_sub(cs + before + m.0 as u32);
                if after >= ideal {
                    if is_reversed && before >= ideal {
                        (cp - ideal as i32 - m.1 as i32, ideal)
                    } else {
                        (cp + cs as i32 + m.0 as i32, ideal)
                    }
                } else if before >= ideal {
                    (cp - ideal as i32 - m.1 as i32, ideal)
                } else if before > after {
                    (rp, before)
                } else {
                    (cp + cs as i32 + m.0 as i32, after)
                }
            };
            let place_out = |rp, rs, cp: i32, cs, ideal: u32| -> (i32, u32) {
                let pos = cp.min(rp + rs as i32 - ideal as i32).max(rp);
                let size = ideal.max(cs).min(rs);
                (pos, size)
            };
            let rect = if popup.direction.is_horizontal() {
                let (x, w) = place_in(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0, m.horiz);
                let (y, h) = place_out(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1);
                Rect::new(Coord(x, y), Size(w, h))
            } else {
                let (x, w) = place_out(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0);
                let (y, h) = place_in(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1, m.vert);
                Rect::new(Coord(x, y), Size(w, h))
            };

            cache.apply_rect(widget, size_handle, rect, false);
        }
    }

    fn handle_closure(&mut self, mgr: &mut Manager) {
        for (condition, f) in &self.fns {
            match condition {
                Callback::Close => f(&mut self.w, mgr),
                Callback::Start => (),
            }
        }
    }
}
