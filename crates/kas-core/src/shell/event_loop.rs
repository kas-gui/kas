// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event loop and handling

use std::collections::HashMap;
use std::time::{Duration, Instant};

use winit::event::{Event, StartCause};
use winit::event_loop::{ControlFlow, EventLoopWindowTarget};
use winit::window as ww;

use super::{Pending, SharedState};
use super::{ProxyAction, Window, WindowSurface};
use kas::theme::Theme;
use kas::{Action, AppData, WindowId};

/// Event-loop data structure (i.e. all run-time state)
pub(super) struct Loop<A: AppData, S: WindowSurface, T: Theme<S::Shared>>
where
    T::Window: kas::theme::Window,
{
    /// State is suspended until we receive Event::Resumed
    suspended: bool,
    /// Window states
    windows: HashMap<WindowId, Box<Window<A, S, T>>>,
    popups: HashMap<WindowId, WindowId>,
    /// Translates our WindowId to winit's
    id_map: HashMap<ww::WindowId, WindowId>,
    /// Shared data passed from Toolkit
    shared: SharedState<A, S, T>,
    /// Timer resumes: (time, window identifier)
    resumes: Vec<(Instant, WindowId)>,
    /// Frame rate counter
    frame_count: (Instant, u32),
}

impl<A: AppData, S: WindowSurface, T: Theme<S::Shared>> Loop<A, S, T>
where
    T::Window: kas::theme::Window,
{
    pub(super) fn new(
        mut windows: Vec<Box<Window<A, S, T>>>,
        shared: SharedState<A, S, T>,
    ) -> Self {
        Loop {
            suspended: true,
            windows: windows.drain(..).map(|w| (w.window_id, w)).collect(),
            popups: Default::default(),
            id_map: Default::default(),
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
        match event {
            Event::NewEvents(cause) => {
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

            Event::WindowEvent { window_id, event } => {
                self.flush_pending(elwt, control_flow);

                if let Some(id) = self.id_map.get(&window_id) {
                    if let Some(window) = self.windows.get_mut(id) {
                        window.handle_event(&mut self.shared, event);
                    }
                }
            }
            Event::DeviceEvent { .. } => {
                // windows handle local input; we do not handle global input
            }
            Event::UserEvent(action) => match action {
                ProxyAction::Close(id) => {
                    if let Some(window) = self.windows.get_mut(&id) {
                        window.send_action(Action::CLOSE);
                    }
                }
                ProxyAction::CloseAll => {
                    for window in self.windows.values_mut() {
                        window.send_action(Action::CLOSE);
                    }
                }
                ProxyAction::Message(msg) => {
                    let mut stack = crate::ErasedStack::new();
                    stack.push_erased(msg.into_erased());
                    self.shared.handle_messages(&mut stack);
                }
                ProxyAction::WakeAsync => {
                    // We don't need to do anything: MainEventsCleared will
                    // automatically be called after, which automatically calls
                    // window.update(..), which calls EventState::Update.
                }
            },

            Event::Suspended if !self.suspended => {
                for window in self.windows.values_mut() {
                    window.suspend();
                }
                self.suspended = true;
            }
            Event::Suspended => (),
            Event::Resumed if self.suspended => {
                for window in self.windows.values_mut() {
                    match window.resume(&mut self.shared, elwt) {
                        Ok(winit_id) => {
                            self.id_map.insert(winit_id, window.window_id);
                        }
                        Err(e) => {
                            log::error!("Unable to create window: {}", e);
                        }
                    }
                }
                self.suspended = false;
            }
            Event::Resumed => (),

            Event::AboutToWait => {
                self.flush_pending(elwt, control_flow);
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

            Event::RedrawRequested(id) => {
                // We must conclude pending actions (such as resize) before drawing.
                self.flush_pending(elwt, control_flow);

                if let Some(id) = self.id_map.get(&id) {
                    if let Some(window) = self.windows.get_mut(id) {
                        if window.do_draw(&mut self.shared).is_err() {
                            *control_flow = ControlFlow::Poll;
                        }
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

            Event::LoopExiting => (),
        }
    }

    fn flush_pending(
        &mut self,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        control_flow: &mut ControlFlow,
    ) {
        while let Some(pending) = self.shared.shell.pending.pop_front() {
            match pending {
                Pending::AddPopup(parent_id, id, popup) => {
                    log::debug!("Pending: adding overlay");
                    // TODO: support pop-ups as a special window, where available
                    self.windows.get_mut(&parent_id).unwrap().add_popup(
                        &mut self.shared,
                        id,
                        popup,
                    );
                    self.popups.insert(id, parent_id);
                }
                Pending::AddWindow(id, mut window) => {
                    log::debug!("Pending: adding window {}", window.widget.title());
                    if !self.suspended {
                        match window.resume(&mut self.shared, elwt) {
                            Ok(winit_id) => {
                                self.id_map.insert(winit_id, id);
                            }
                            Err(e) => {
                                log::error!("Unable to create window: {}", e);
                            }
                        }
                    }
                    self.windows.insert(id, window);
                }
                Pending::CloseWindow(target) => {
                    let mut win_id = target;
                    if let Some(id) = self.popups.remove(&target) {
                        win_id = id;
                    }
                    if let Some(window) = self.windows.get_mut(&win_id) {
                        window.send_close(&mut self.shared, target);
                    }
                }
                Pending::Action(action) => {
                    if action.contains(Action::CLOSE | Action::EXIT) {
                        self.windows.clear();
                        self.id_map.clear();
                        *control_flow = ControlFlow::Poll;
                    } else {
                        for (_, window) in self.windows.iter_mut() {
                            window.handle_action(&mut self.shared, action);
                        }
                    }
                }
            }
        }

        let mut close_all = false;
        self.resumes.clear();
        self.windows.retain(|window_id, window| {
            let (action, resume) = window.flush_pending(&mut self.shared);
            if let Some(instant) = resume {
                self.resumes.push((instant, *window_id));
            }
            if action.contains(Action::EXIT) {
                close_all = true;
                true
            } else if action.contains(Action::CLOSE) {
                self.id_map.retain(|_, v| v != window_id);
                false
            } else {
                true
            }
        });

        if close_all {
            for (_, mut window) in self.windows.drain() {
                window.suspend();
            }
            self.id_map.clear();
        }
    }
}
