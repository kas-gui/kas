// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: touch events

use super::{GrabMode, Press, PressSource, velocity};
use crate::config::EventWindowConfig;
use crate::event::{Event, EventCx, EventState, FocusSource, NavAdvance, PressStart};
use crate::geom::{Affine, DVec2, Vec2};
use crate::window::Window;
use crate::{Action, Id, Layout, Node, Widget};
use cast::{Cast, CastApprox, CastFloat, Conv};
use smallvec::SmallVec;
use winit::event::TouchPhase;

const MAX_TOUCHES: usize = 10;
const MAX_PANS: usize = 2;
const VELOCITY_LEN: usize = 2;
const MAX_PAN_GRABS: usize = 2;

#[derive(Clone, Debug)]
pub(super) struct TouchGrab {
    id: u64,
    pub(super) start_id: Id,
    pub(super) depress: Option<Id>,
    over: Option<Id>,
    last_position: DVec2,
    mode: GrabMode,
    pan_grab: (u16, u16),
    vel_index: u16,
    cancel: bool,
}

impl TouchGrab {
    fn flush_click_move(&mut self) -> Action {
        if self.mode == GrabMode::Click {
            if self.start_id == self.over {
                if self.depress != self.over {
                    self.depress = self.over.clone();
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
    mode: (bool, bool), // (scale, rotate)
    n: u16,
    coords: [(DVec2, DVec2); MAX_PAN_GRABS],
}

#[derive(Default)]
pub(crate) struct Touch {
    pub(super) touch_grab: SmallVec<[TouchGrab; MAX_TOUCHES]>,
    pan_grab: SmallVec<[PanGrab; MAX_PANS]>,
    velocity: [velocity::Samples; VELOCITY_LEN],
}

impl Touch {
    /// `mode` is `(scale, rotate)`
    pub(super) fn set_pan_on(&mut self, id: Id, mode: (bool, bool), p: DVec2) -> (u16, u16) {
        for (gi, grab) in self.pan_grab.iter_mut().enumerate() {
            if grab.id == id {
                debug_assert_eq!(grab.mode, mode);

                let index = grab.n;
                if usize::from(index) < MAX_PAN_GRABS {
                    grab.coords[usize::from(index)] = (p, p);
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
        let mut coords: [(DVec2, DVec2); MAX_PAN_GRABS] = Default::default();
        coords[0] = (p, p);
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
                    let p = grab.last_position;
                    self.pan_grab[usize::from(g.0)].coords[usize::from(grab.pan_grab.1)] = (p, p);
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
        position: DVec2,
        mode: GrabMode,
    ) -> bool {
        let mut velocity = u16::MAX;
        if mode == GrabMode::Grab {
            let mut used = [false; VELOCITY_LEN];
            for grab in &self.touch_grab {
                if (grab.vel_index as usize) < VELOCITY_LEN {
                    used[grab.vel_index as usize] = true;
                }
            }

            for i in 0..VELOCITY_LEN {
                if !used[i] {
                    self.velocity[i].clear();
                    velocity = i as u16;
                    break;
                }
            }
        }

        if let Some(grab) = self.get_touch(touch_id) {
            if grab.start_id != id || grab.mode != mode || grab.cancel {
                return false;
            }

            grab.depress = Some(id.clone());
            grab.over = Some(id.clone());
            grab.last_position = position;
            grab.vel_index = velocity;
            true
        } else if self.touch_grab.len() < MAX_TOUCHES {
            let mut pan_grab = (u16::MAX, 0);
            if let GrabMode::Pan { scale, rotate } = mode {
                pan_grab = self.set_pan_on(id.clone(), (scale, rotate), position);
            }

            self.touch_grab.push(TouchGrab {
                id: touch_id,
                start_id: id.clone(),
                depress: Some(id.clone()),
                over: Some(id.clone()),
                last_position: position,
                mode,
                pan_grab,
                vel_index: velocity,
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
            .map(|grab| grab.vel_index)
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
        self.window_action(grab.flush_click_move());
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
                    source: PressSource::touch(grab.id),
                    id: grab.over,
                    coord: grab.last_position.cast_nearest(),
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
                grab.over = win.try_probe(grab.last_position.cast_nearest());
            }
        }
    }

    pub(in crate::event::cx) fn touch_frame_update(&mut self, mut node: Node<'_>) {
        for gi in 0..self.touch.pan_grab.len() {
            let grab = &mut self.touch.pan_grab[gi];
            assert!(grab.n > 0);

            // Terminology: pi are old coordinates, qi are new coords
            let (p1, q1) = (grab.coords[0].0, grab.coords[0].1);
            grab.coords[0].0 = grab.coords[0].1;

            let transform = if grab.n == 1 {
                Affine::translate(q1 - p1)
            } else {
                // Only use the first two touches (we don't need more info)
                let (p2, q2) = (grab.coords[1].0, grab.coords[1].1);
                grab.coords[1].0 = grab.coords[1].1;
                Affine::pan(p1, q1, p2, q2, grab.mode)
            };

            let id = grab.id.clone();
            if transform.is_finite() && transform != Affine::IDENTITY {
                let event = Event::Pan(transform);
                self.send_event(node.re(), id, event);
            }
        }
    }

    pub(in crate::event::cx) fn handle_touch_event<A>(
        &mut self,
        win: &mut Window<A>,
        data: &A,
        touch: winit::event::Touch,
    ) {
        let source = PressSource::touch(touch.id);
        let position: DVec2 = touch.location.into();
        let coord = position.cast_nearest();
        match touch.phase {
            TouchPhase::Started => {
                let over = win.try_probe(coord);
                self.close_non_ancestors_of(over.as_ref());

                if let Some(id) = over {
                    if self.config.event().touch_nav_focus()
                        && let Some(id) =
                            self.nav_next(win.as_node(data), Some(&id), NavAdvance::None)
                    {
                        self.set_nav_focus(id, FocusSource::Pointer);
                    }

                    let press = PressStart {
                        source,
                        id: Some(id.clone()),
                        position,
                    };
                    let event = Event::PressStart(press);
                    self.send_event(win.as_node(data), id, event);
                }
            }
            TouchPhase::Moved => {
                let over = win.try_probe(coord);

                let mut pan_grab = None;
                let grab_index = self
                    .touch
                    .touch_grab
                    .iter()
                    .enumerate()
                    .find_map(|(i, grab)| (grab.id == touch.id).then_some(i));
                if let Some(index) = grab_index {
                    let last_pos = std::mem::replace(
                        &mut self.touch.touch_grab[index].last_position,
                        position,
                    );
                    let delta: Vec2 = (position - last_pos).cast_approx();

                    let vi = self.touch.touch_grab[index].vel_index as usize;
                    if vi < VELOCITY_LEN {
                        self.touch.velocity[vi].push_delta(delta);
                    }

                    let grab = &mut self.touch.touch_grab[index];
                    grab.over = over;

                    match grab.mode {
                        GrabMode::Click => {}
                        GrabMode::Grab => {
                            let target = grab.start_id.clone();
                            let press = Press {
                                source: PressSource::touch(grab.id),
                                id: grab.over.clone(),
                                coord,
                            };
                            let event = Event::PressMove { press, delta };
                            self.send_event(win.as_node(data), target, event);
                        }
                        GrabMode::Pan { .. } => {
                            pan_grab = Some(grab.pan_grab);
                        }
                    }
                }

                if let Some(pan_grab) = pan_grab {
                    self.need_frame_update = true;
                    if usize::conv(pan_grab.1) < MAX_PAN_GRABS
                        && let Some(pan) = self.touch.pan_grab.get_mut(usize::conv(pan_grab.0))
                    {
                        pan.coords[usize::conv(pan_grab.1)].1 = position;
                    }
                }
            }
            ev @ (TouchPhase::Ended | TouchPhase::Cancelled) => {
                if let Some(index) = self.touch.get_touch_index(touch.id) {
                    let mut to_send = None;
                    if let Some(grab) = self.touch.touch_grab.get(index)
                        && !grab.mode.is_pan()
                    {
                        let id = grab.over.clone();
                        let press = Press { source, id, coord };
                        let success = ev == TouchPhase::Ended;

                        let event = Event::PressEnd { press, success };
                        to_send = Some((grab.start_id.clone(), event));
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
