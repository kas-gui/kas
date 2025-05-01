// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: touch events

use super::{velocity, GrabMode, Press, PressSource};
use crate::config::EventWindowConfig;
use crate::event::{Event, EventCx, EventState, FocusSource};
use crate::geom::{Coord, DVec2, Vec2};
use crate::{Action, Id, NavAdvance, Node, Widget, Window};
use cast::{Cast, CastApprox, Conv};
use smallvec::SmallVec;
use winit::event::TouchPhase;

const MAX_TOUCHES: usize = 10;
const MAX_PANS: usize = 2;
const MAX_VELOCITY: usize = 2;
const MAX_PAN_GRABS: usize = 2;

#[derive(Clone, Debug)]
pub(super) struct TouchGrab {
    id: u64,
    pub(super) start_id: Id,
    pub(super) depress: Option<Id>,
    cur_id: Option<Id>,
    last_move: Coord,
    coord: Coord,
    last_position: Vec2,
    mode: GrabMode,
    pan_grab: (u16, u16),
    velocity: u16,
    cancel: bool,
}

impl TouchGrab {
    fn flush_click_move(&mut self) -> Action {
        if self.mode == GrabMode::Click && self.last_move != self.coord {
            self.last_move = self.coord;
            if self.start_id == self.cur_id {
                if self.depress != self.cur_id {
                    self.depress = self.cur_id.clone();
                    return Action::REDRAW;
                }
            } else if self.depress.is_some() {
                self.depress = None;
                return Action::REDRAW;
            }
        }
        Action::empty()
    }
}

#[derive(Clone, Debug)]
struct PanGrab {
    id: Id,
    mode: GrabMode,
    n: u16,
    coords: [(Coord, Coord); MAX_PAN_GRABS],
}

#[derive(Default)]
pub(in crate::event::cx) struct Touch {
    pub(super) touch_grab: SmallVec<[TouchGrab; MAX_TOUCHES]>,
    pan_grab: SmallVec<[PanGrab; MAX_PANS]>,
    velocity: [velocity::Samples; MAX_VELOCITY],
}

impl Touch {
    pub(super) fn set_pan_on(&mut self, id: Id, mode: GrabMode, coord: Coord) -> (u16, u16) {
        for (gi, grab) in self.pan_grab.iter_mut().enumerate() {
            if grab.id == id {
                debug_assert_eq!(grab.mode, mode);

                let index = grab.n;
                if usize::from(index) < MAX_PAN_GRABS {
                    grab.coords[usize::from(index)] = (coord, coord);
                }
                grab.n = index + 1;
                return (gi.cast(), index);
            }
        }

        if self.pan_grab.len() >= MAX_PANS {
            return (u16::MAX, 0);
        }

        let gj = self.pan_grab.len().cast();
        let n = 1;
        let mut coords: [(Coord, Coord); MAX_PAN_GRABS] = Default::default();
        coords[0] = (coord, coord);
        log::trace!("set_pan_on: index={}, id={id}", self.pan_grab.len());
        self.pan_grab.push(PanGrab {
            id,
            mode,
            n,
            coords,
        });
        (gj, 0)
    }

    fn remove_pan(&mut self, index: usize) {
        log::trace!("remove_pan: index={index}");
        self.pan_grab.remove(index);
        for grab in self.touch_grab.iter_mut() {
            let p0 = grab.pan_grab.0;
            if usize::from(p0) >= index && p0 != u16::MAX {
                grab.pan_grab.0 = p0 - 1;
            }
        }
    }

    fn remove_pan_grab(&mut self, g: (u16, u16)) {
        if let Some(grab) = self.pan_grab.get_mut(usize::from(g.0)) {
            grab.n -= 1;
            if grab.n == 0 {
                return self.remove_pan(g.0.into());
            }
            for i in (usize::from(g.1))..(usize::from(grab.n) - 1) {
                grab.coords[i] = grab.coords[i + 1];
            }
        } else {
            return;
        }

        for grab in self.touch_grab.iter_mut() {
            if grab.pan_grab.0 == g.0 && grab.pan_grab.1 > g.1 {
                grab.pan_grab.1 -= 1;
                if usize::from(grab.pan_grab.1) == MAX_PAN_GRABS - 1 {
                    let v = grab.coord;
                    self.pan_grab[usize::from(g.0)].coords[usize::from(grab.pan_grab.1)] = (v, v);
                }
            }
        }
    }

    pub(in crate::event::cx) fn cancel_event_focus(&mut self, target: &Id) {
        for grab in self.touch_grab.iter_mut() {
            if grab.start_id == target {
                grab.cancel = true;
            }
        }
    }

    #[inline]
    fn get_touch_index(&self, touch_id: u64) -> Option<usize> {
        self.touch_grab
            .iter()
            .enumerate()
            .find_map(|(i, grab)| (grab.id == touch_id).then_some(i))
    }

    #[inline]
    pub(super) fn get_touch(&mut self, touch_id: u64) -> Option<&mut TouchGrab> {
        self.touch_grab.iter_mut().find(|grab| grab.id == touch_id)
    }

    /// Returns `true` on success
    pub(super) fn start_grab(
        &mut self,
        touch_id: u64,
        id: Id,
        coord: Coord,
        mode: GrabMode,
    ) -> bool {
        let mut velocity = u16::MAX;
        if mode == GrabMode::Grab {
            let mut used = [false; MAX_VELOCITY];
            for grab in &self.touch_grab {
                if (grab.velocity as usize) < MAX_VELOCITY {
                    used[grab.velocity as usize] = true;
                }
            }

            for i in 0..MAX_VELOCITY {
                if !used[i] {
                    self.velocity[i].clear();
                    velocity = i as u16;
                    break;
                }
            }
        }

        if let Some(grab) = self.get_touch(touch_id) {
            if grab.mode.is_pan() != mode.is_pan() || grab.cancel {
                return false;
            }

            grab.depress = Some(id.clone());
            grab.cur_id = Some(id.clone());
            grab.last_move = coord;
            grab.coord = coord;
            grab.mode = grab.mode.max(mode);
            grab.velocity = velocity;
            true
        } else if self.touch_grab.len() < MAX_TOUCHES {
            let mut pan_grab = (u16::MAX, 0);
            if mode.is_pan() {
                pan_grab = self.set_pan_on(id.clone(), mode, coord);
            }

            self.touch_grab.push(TouchGrab {
                id: touch_id,
                start_id: id.clone(),
                depress: Some(id.clone()),
                cur_id: Some(id.clone()),
                last_move: coord,
                coord,
                last_position: coord.cast(),
                mode,
                pan_grab,
                velocity,
                cancel: false,
            });
            true
        } else {
            false
        }
    }

    pub(super) fn velocity(&self, touch_id: u64, evc: EventWindowConfig<'_>) -> Option<Vec2> {
        let v = self
            .touch_grab
            .iter()
            .find(|grab| grab.id == touch_id)
            .map(|grab| grab.velocity)
            .unwrap_or(u16::MAX);
        self.velocity
            .get(v as usize)
            .map(|sampler| sampler.velocity(evc.kinetic_timeout()))
    }
}

impl EventState {
    // Clears touch grab and pan grab and redraws
    //
    // Returns the grab. Panics on out-of-bounds error.
    fn remove_touch(&mut self, index: usize) -> TouchGrab {
        let mut grab = self.touch.touch_grab.remove(index);
        log::trace!(
            "remove_touch: touch_id={}, start_id={}",
            grab.id,
            grab.start_id
        );
        self.opt_action(grab.depress.clone(), Action::REDRAW);
        self.touch.remove_pan_grab(grab.pan_grab);
        self.action(Id::ROOT, grab.flush_click_move());
        grab
    }
}

impl<'a> EventCx<'a> {
    pub(in crate::event::cx) fn touch_handle_pending<A>(&mut self, win: &mut Window<A>, data: &A) {
        let mut i = 0;
        while i < self.touch.touch_grab.len() {
            let action = self.touch.touch_grab[i].flush_click_move();
            self.state.action |= action;

            if self.touch.touch_grab[i].cancel {
                let grab = self.remove_touch(i);

                let press = Press {
                    source: PressSource::Touch(grab.id),
                    id: grab.cur_id,
                    coord: grab.coord,
                };
                let event = Event::PressEnd {
                    press,
                    success: false,
                };
                self.send_event(win.as_node(data), grab.start_id, event);
            } else {
                i += 1;
            }
        }

        if self.action.contains(Action::REGION_MOVED) {
            for grab in self.touch.touch_grab.iter_mut() {
                grab.cur_id = win.try_probe(grab.coord);
            }
        }
    }

    pub(in crate::event::cx) fn touch_frame_update(&mut self, mut node: Node<'_>) {
        for gi in 0..self.touch.pan_grab.len() {
            let grab = &mut self.touch.pan_grab[gi];
            debug_assert!(grab.mode != GrabMode::Grab);
            assert!(grab.n > 0);

            // Terminology: pi are old coordinates, qi are new coords
            let (p1, q1) = (DVec2::conv(grab.coords[0].0), DVec2::conv(grab.coords[0].1));
            grab.coords[0].0 = grab.coords[0].1;

            let alpha;
            let delta;

            if grab.mode == GrabMode::PanOnly || grab.n == 1 {
                alpha = DVec2(1.0, 0.0);
                delta = q1 - p1;
            } else {
                // We don't use more than two touches: information would be
                // redundant (although it could be averaged).
                let (p2, q2) = (DVec2::conv(grab.coords[1].0), DVec2::conv(grab.coords[1].1));
                grab.coords[1].0 = grab.coords[1].1;
                let (pd, qd) = (p2 - p1, q2 - q1);

                alpha = match grab.mode {
                    GrabMode::PanFull => qd.complex_div(pd),
                    GrabMode::PanScale => DVec2((qd.sum_square() / pd.sum_square()).sqrt(), 0.0),
                    GrabMode::PanRotate => {
                        let a = qd.complex_div(pd);
                        a / a.sum_square().sqrt()
                    }
                    _ => unreachable!(),
                };

                // Average delta from both movements:
                delta = (q1 - alpha.complex_mul(p1) + q2 - alpha.complex_mul(p2)) * 0.5;
            }

            let id = grab.id.clone();
            if alpha.is_finite()
                && delta.is_finite()
                && (alpha != DVec2(1.0, 0.0) || delta != DVec2::ZERO)
            {
                let event = Event::Pan { alpha, delta };
                self.send_event(node.re(), id, event);
            }
        }
    }

    #[cfg(winit)]
    pub(in crate::event::cx) fn handle_touch_event<A>(
        &mut self,
        win: &mut Window<A>,
        data: &A,
        touch: winit::event::Touch,
    ) {
        let source = PressSource::Touch(touch.id);
        let coord = touch.location.cast_approx();
        match touch.phase {
            TouchPhase::Started => {
                let start_id = win.try_probe(coord);
                if let Some(id) = start_id.as_ref() {
                    if self.config.event().touch_nav_focus() {
                        if let Some(id) =
                            self.nav_next(win.as_node(data), Some(id), NavAdvance::None)
                        {
                            self.set_nav_focus(id, FocusSource::Pointer);
                        }
                    }

                    let press = Press {
                        source,
                        id: start_id.clone(),
                        coord,
                    };
                    let event = Event::PressStart { press };
                    self.send_popup_first(win.as_node(data), start_id, event);
                }
            }
            TouchPhase::Moved => {
                let cur_id = win.try_probe(coord);

                let mut redraw = false;
                let mut pan_grab = None;
                let grab_index = self
                    .touch
                    .touch_grab
                    .iter()
                    .enumerate()
                    .find_map(|(i, grab)| (grab.id == touch.id).then_some(i));
                if let Some(index) = grab_index {
                    let v = self.touch.touch_grab[index].velocity as usize;
                    if v < MAX_VELOCITY {
                        let position = DVec2::from(touch.location).cast_approx();
                        let last_vel = std::mem::replace(
                            &mut self.touch.touch_grab[index].last_position,
                            position,
                        );
                        self.touch.velocity[v].push_delta(position - last_vel);
                    }

                    let grab = &mut self.touch.touch_grab[index];
                    if grab.mode == GrabMode::Grab {
                        // Only when 'depressed' status changes:
                        redraw = grab.cur_id != cur_id
                            && (grab.start_id == grab.cur_id || grab.start_id == cur_id);

                        grab.cur_id = cur_id;
                        grab.coord = coord;

                        if grab.last_move != grab.coord {
                            let delta = grab.coord - grab.last_move;
                            let target = grab.start_id.clone();
                            let press = Press {
                                source: PressSource::Touch(grab.id),
                                id: grab.cur_id.clone(),
                                coord: grab.coord,
                            };
                            let event = Event::PressMove { press, delta };
                            grab.last_move = grab.coord;
                            self.send_event(win.as_node(data), target, event);
                        }
                    } else {
                        pan_grab = Some(grab.pan_grab);
                    }
                }

                if redraw {
                    self.window_action(Action::REDRAW);
                } else if let Some(pan_grab) = pan_grab {
                    self.need_frame_update = true;
                    if usize::conv(pan_grab.1) < MAX_PAN_GRABS {
                        if let Some(pan) = self.touch.pan_grab.get_mut(usize::conv(pan_grab.0)) {
                            pan.coords[usize::conv(pan_grab.1)].1 = coord;
                        }
                    }
                }
            }
            ev @ (TouchPhase::Ended | TouchPhase::Cancelled) => {
                if let Some(index) = self.touch.get_touch_index(touch.id) {
                    let mut to_send = None;
                    if let Some(grab) = self.touch.touch_grab.get(index) {
                        if !grab.mode.is_pan() {
                            let id = grab.cur_id.clone();
                            let press = Press { source, id, coord };
                            let success = ev == TouchPhase::Ended;

                            let event = Event::PressEnd { press, success };
                            to_send = Some((grab.start_id.clone(), event));
                        }
                    }

                    // We must send Event::PressEnd before removing the grab
                    if let Some((id, event)) = to_send {
                        self.send_event(win.as_node(data), id, event);
                    }

                    self.remove_touch(index);
                }
            }
        }
    }
}
