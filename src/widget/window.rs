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

/// The main instantiation of the [`Window`] trait.
#[handler(action, generics = <> where W: Widget<Msg = VoidMsg>)]
#[widget(children=noauto)]
#[derive(Widget)]
pub struct Window<W: Widget + 'static> {
    #[widget_core]
    core: CoreData,
    restrict_dimensions: (bool, bool),
    title: CowString,
    #[widget]
    w: W,
    popups: Vec<kas::Popup>,
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
                .map(|popup| popup.overlay.as_widget())
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        if index == 0 {
            Some(&mut self.w)
        } else {
            self.popups
                .get_mut(index - 1)
                .map(|popup| popup.overlay.as_widget_mut())
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
    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.w.set_rect(size_handle, rect, align);
        for popup in &mut self.popups {
            let ir = self.w.find(popup.parent).unwrap().rect();
            let widget = popup.overlay.as_widget_mut();
            let (_, ideal) = layout::solve(widget, size_handle);

            let place_in = |p_out, s_out, p_in, s_in, ideal| -> (i32, u32) {
                debug_assert!(p_in >= p_out);
                let before = (p_in - p_out) as u32;
                debug_assert!(s_out >= s_in + before);
                let after = s_out - s_in - before;
                if after >= ideal {
                    (p_in + s_in as i32, ideal)
                } else if before >= ideal {
                    (p_in - ideal as i32, ideal)
                } else if before > after {
                    (p_out, before)
                } else {
                    (p_in + s_in as i32, after)
                }
            };
            let place_out = |p_out, s_out, p_in: i32, s_in, ideal: u32| -> (i32, u32) {
                let pos = p_in.min(p_out + s_out as i32 - ideal as i32).max(p_out);
                let size = ideal.max(s_in).min(s_out);
                (pos, size)
            };
            let rect = match popup.direction {
                Direction::Horizontal => {
                    let (x, w) = place_in(rect.pos.0, rect.size.0, ir.pos.0, ir.size.0, ideal.0);
                    let (y, h) = place_out(rect.pos.1, rect.size.1, ir.pos.1, ir.size.1, ideal.1);
                    Rect::new(Coord(x, y), Size(w, h))
                }
                Direction::Vertical => {
                    let (x, w) = place_out(rect.pos.0, rect.size.0, ir.pos.0, ir.size.0, ideal.0);
                    let (y, h) = place_in(rect.pos.1, rect.size.1, ir.pos.1, ir.size.1, ideal.1);
                    Rect::new(Coord(x, y), Size(w, h))
                }
            };

            layout::solve_and_set(widget, size_handle, rect, false);
        }
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        for popup in self.popups.iter().rev() {
            if let Some(id) = popup.overlay.find_id(coord) {
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
                popup.overlay.draw(draw_handle, mgr);
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
                let widget = &mut self.popups[i].overlay;
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

    fn add_popup(&mut self, _: &mut dyn SizeHandle, mgr: &mut Manager, popup: kas::Popup) {
        // TODO: using reconfigure here is inefficient
        mgr.send_action(TkAction::Reconfigure);
        self.popups.push(popup);
    }

    fn callbacks(&self) -> Vec<(usize, Callback)> {
        self.fns.iter().map(|(cond, _)| *cond).enumerate().collect()
    }

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, mgr: &mut Manager) {
        let cb = &mut self.fns[index].1;
        cb(&mut self.w, mgr);
    }
}
