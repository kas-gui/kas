// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event loop and handling

use smallvec::SmallVec;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use winit::event::{Event, StartCause};
use winit::event_loop::{ControlFlow, EventLoopWindowTarget};
use winit::window as ww;

use super::{PendingAction, SharedState};
use super::{ProxyAction, Window, WindowSurface};
use kas::theme::Theme;
use kas::{Action, WindowId};

/// Event-loop data structure (i.e. all run-time state)
pub(super) struct Loop<A: 'static, S: WindowSurface, T: Theme<S::Shared>>
where
    T::Window: kas::theme::Window,
{
    /// Window states
    windows: HashMap<ww::WindowId, Window<A, S, T>>,
    /// Translates our WindowId to winit's
    id_map: HashMap<WindowId, ww::WindowId>,
    /// Shared data passed from Toolkit
    shared: SharedState<A, S, T>,
    /// Timer resumes: (time, window index)
    resumes: Vec<(Instant, ww::WindowId)>,
    /// Frame rate counter
    frame_count: (Instant, u32),
}

impl<A, S: WindowSurface, T: Theme<S::Shared>> Loop<A, S, T>
where
    T::Window: kas::theme::Window,
{
    pub(super) fn new(mut windows: Vec<Window<A, S, T>>, shared: SharedState<A, S, T>) -> Self {
        let id_map = windows
            .iter()
            .map(|w| (w.window_id, w.window.id()))
            .collect();
        Loop {
            windows: windows.drain(..).map(|w| (w.window.id(), w)).collect(),
            id_map,
            shared,
            resumes: vec![],
            frame_count: (Instant::now(), 0),
        }
    }

    pub(super) fn handle(
        &mut self,
        event: Event<ProxyAction>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        control_flow: &mut ControlFlow,
    ) {
        use Event::*;

        match event {
            NewEvents(cause) => {
                // MainEventsCleared will reset control_flow (but not when it is Poll)
                *control_flow = ControlFlow::Wait;

                match cause {
                    StartCause::ResumeTimeReached {
                        requested_resume, ..
                    } => {
                        let item = self
                            .resumes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| panic!("timer wakeup without resume"));
                        assert_eq!(item.0, requested_resume);
                        log::trace!("Wakeup: timer (window={:?})", item.1);

                        let resume = if let Some(w) = self.windows.get_mut(&item.1) {
                            w.update_timer(&mut self.shared)
                        } else {
                            // presumably, some window with active timers was removed
                            None
                        };

                        if let Some(instant) = resume {
                            self.resumes[0].0 = instant;
                        } else {
                            self.resumes.remove(0);
                        }
                    }
                    StartCause::WaitCancelled { .. } => {
                        // This event serves no purpose?
                        // log::debug!("Wakeup: WaitCancelled (ignoring)");
                    }
                    StartCause::Poll => (),
                    StartCause::Init => (),
                }
            }

            WindowEvent { window_id, event } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.handle_event(&mut self.shared, event);
                }
            }
            DeviceEvent { .. } => {
                // windows handle local input; we do not handle global input
            }
            UserEvent(action) => match action {
                ProxyAction::Close(id) => {
                    if let Some(id) = self.id_map.get(&id) {
                        if let Some(window) = self.windows.get_mut(id) {
                            window.send_action(Action::CLOSE);
                        }
                    }
                }
                ProxyAction::CloseAll => {
                    for window in self.windows.values_mut() {
                        window.send_action(Action::CLOSE);
                    }
                }
                ProxyAction::Update(handle, payload) => {
                    self.shared
                        .pending
                        .push(PendingAction::Update(handle, payload));
                }
                ProxyAction::WakeAsync => {
                    // We don't need to do anything: MainEventsCleared will
                    // automatically be called after, which automatically calls
                    // window.update(..), which calls EventState::Update.
                }
            },

            // TODO: windows should be constructed in Resumed and destroyed
            // (everything but the widget) in Suspended:
            Suspended => (),
            Resumed => (),

            MainEventsCleared => {
                while let Some(pending) = self.shared.pending.pop() {
                    match pending {
                        PendingAction::AddPopup(parent_id, id, popup) => {
                            log::debug!("Pending: adding overlay");
                            // TODO: support pop-ups as a special window, where available
                            self.windows.get_mut(&parent_id).unwrap().add_popup(
                                &mut self.shared,
                                id,
                                popup,
                            );
                            self.id_map.insert(id, parent_id);
                        }
                        PendingAction::AddWindow(id, widget) => {
                            log::debug!("Pending: adding window {}", widget.title());
                            match Window::new(&mut self.shared, elwt, id, widget) {
                                Ok(window) => {
                                    let wid = window.window.id();
                                    self.id_map.insert(id, wid);
                                    self.windows.insert(wid, window);
                                }
                                Err(e) => {
                                    log::error!("Unable to create window: {}", e);
                                }
                            };
                        }
                        PendingAction::CloseWindow(id) => {
                            if let Some(wwid) = self.id_map.get(&id) {
                                if let Some(window) = self.windows.get_mut(wwid) {
                                    window.send_close(&mut self.shared, id);
                                }
                                self.id_map.remove(&id);
                            }
                        }
                        PendingAction::Action(action) => {
                            if action.contains(Action::CLOSE | Action::EXIT) {
                                self.windows.clear();
                                *control_flow = ControlFlow::Poll;
                            } else {
                                for (_, window) in self.windows.iter_mut() {
                                    window.handle_action(&mut self.shared, action);
                                }
                            }
                        }
                        PendingAction::Update(handle, payload) => {
                            for window in self.windows.values_mut() {
                                window.update_widgets(&mut self.shared, handle, payload);
                            }
                        }
                    }
                }

                let mut close_all = false;
                let mut to_close = SmallVec::<[ww::WindowId; 4]>::new();
                self.resumes.clear();
                for (window_id, window) in self.windows.iter_mut() {
                    let (action, resume) = window.update(&mut self.shared);
                    if action.contains(Action::EXIT) {
                        close_all = true;
                    } else if action.contains(Action::CLOSE) {
                        to_close.push(*window_id);
                    }
                    if let Some(instant) = resume {
                        self.resumes.push((instant, *window_id));
                    }
                }

                for window_id in &to_close {
                    if let Some(window) = self.windows.remove(window_id) {
                        self.id_map.remove(&window.window_id);
                    }
                }
                if close_all {
                    self.windows.clear();
                }

                self.resumes.sort_by_key(|item| item.0);

                let is_exit = matches!(control_flow, ControlFlow::ExitWithCode(_));
                *control_flow = if is_exit || self.windows.is_empty() {
                    self.shared.on_exit();
                    debug_assert!(!is_exit || matches!(control_flow, ControlFlow::ExitWithCode(0)));
                    ControlFlow::ExitWithCode(0)
                } else if *control_flow == ControlFlow::Poll {
                    ControlFlow::Poll
                } else if let Some((instant, _)) = self.resumes.first() {
                    ControlFlow::WaitUntil(*instant)
                } else {
                    ControlFlow::Wait
                };
            }

            RedrawRequested(id) => {
                if let Some(window) = self.windows.get_mut(&id) {
                    if window.do_draw(&mut self.shared).is_err() {
                        *control_flow = ControlFlow::Poll;
                    }
                }

                const SECOND: Duration = Duration::from_secs(1);
                self.frame_count.1 += 1;
                let now = Instant::now();
                if self.frame_count.0 + SECOND <= now {
                    log::debug!("Frame rate: {} per second", self.frame_count.1);
                    self.frame_count.0 = now;
                    self.frame_count.1 = 0;
                }
            }
            RedrawEventsCleared => {
                if matches!(control_flow, ControlFlow::Wait | ControlFlow::WaitUntil(_)) {
                    self.resumes.clear();
                    for (window_id, window) in self.windows.iter_mut() {
                        if let Some(instant) = window.post_draw(&mut self.shared) {
                            self.resumes.push((instant, *window_id));
                        }
                    }
                    self.resumes.sort_by_key(|item| item.0);

                    *control_flow = match self.resumes.first() {
                        Some((instant, _)) => ControlFlow::WaitUntil(*instant),
                        None => ControlFlow::Wait,
                    };
                }
            }

            LoopDestroyed => (),
        }
    }
}
