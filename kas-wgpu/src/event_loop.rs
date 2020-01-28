// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event loop and handling

use log::{debug, error, trace};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::time::Instant;

use winit::event::{Event, StartCause};
use winit::event_loop::{ControlFlow, EventLoopWindowTarget};
use winit::window as ww;

use kas::{theme, TkAction};

use crate::draw::DrawPipe;
use crate::shared::{PendingAction, SharedState};
use crate::{ProxyAction, Window, WindowId};

/// Event-loop data structure (i.e. all run-time state)
pub(crate) struct Loop<T: theme::Theme<DrawPipe>> {
    /// Window states
    windows: HashMap<ww::WindowId, Window<T::Window>>,
    /// Translates our WindowId to winit's
    id_map: HashMap<WindowId, ww::WindowId>,
    /// Shared data passed from Toolkit
    shared: SharedState<T>,
    /// Timer resumes: (time, window index)
    resumes: Vec<(Instant, ww::WindowId)>,
}

impl<T: theme::Theme<DrawPipe>> Loop<T> {
    pub(crate) fn new(
        mut windows: Vec<(WindowId, Window<T::Window>)>,
        shared: SharedState<T>,
    ) -> Self {
        let id_map = windows.iter().map(|(id, w)| (*id, w.window.id())).collect();
        Loop {
            windows: windows.drain(..).map(|(_, w)| (w.window.id(), w)).collect(),
            id_map,
            shared,
            resumes: vec![],
        }
    }

    pub(crate) fn handle(
        &mut self,
        event: Event<ProxyAction>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        control_flow: &mut ControlFlow,
    ) {
        use Event::*;

        // In most cases actions.len() is 0 or 1.
        let mut actions = SmallVec::<[_; 2]>::new();
        let mut have_new_resumes = false;

        match event {
            WindowEvent { window_id, event } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    let action = window.handle_event(&mut self.shared, event);
                    actions.push((window_id, action));
                }
            }

            DeviceEvent { .. } => return, // windows handle local input; we do not handle global input
            UserEvent(action) => match action {
                ProxyAction::Close(id) => {
                    if let Some(id) = self.id_map.get(&id) {
                        actions.push((*id, TkAction::Close));
                    }
                }
                ProxyAction::CloseAll => {
                    if let Some(id) = self.windows.keys().next() {
                        // Any id will do; if we have no windows we close anyway!
                        actions.push((*id, TkAction::CloseAll));
                    }
                }
            },

            NewEvents(cause) => {
                // In all cases, we reset control_flow at end of this fn
                *control_flow = ControlFlow::Wait;
                have_new_resumes = true;

                match cause {
                    StartCause::ResumeTimeReached {
                        requested_resume, ..
                    } => {
                        debug!("Wakeup: timer (requested: {:?})", requested_resume);

                        let item = self
                            .resumes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| panic!("timer wakeup without resume"));
                        assert_eq!(item.0, requested_resume);

                        let resume = if let Some(w) = self.windows.get_mut(&item.1) {
                            let (action, resume) =
                                w.timer_resume(&mut self.shared, requested_resume);
                            actions.push((item.1, action));
                            resume
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
                        debug!("Wakeup: WaitCancelled (ignoring)");
                    }
                    StartCause::Poll => {
                        // We use this to check pending actions after removing windows
                    }
                    StartCause::Init => {
                        debug!("Wakeup: init");

                        for (id, window) in self.windows.iter_mut() {
                            let action = window.init(&mut self.shared);
                            actions.push((*id, action));
                        }
                    }
                }
            }

            RedrawRequested(id) => {
                if let Some(window) = self.windows.get_mut(&id) {
                    window.do_draw(&mut self.shared);
                }
            }

            MainEventsCleared | RedrawEventsCleared | LoopDestroyed | Suspended | Resumed => return,
        };

        // Create and init() any new windows.
        while let Some(pending) = self.shared.pending.pop() {
            match pending {
                PendingAction::AddWindow(id, widget) => {
                    debug!("Adding window {}", widget.title());
                    match winit::window::Window::new(elwt) {
                        Ok(wwin) => {
                            wwin.set_title(widget.title());
                            let mut window = Window::new(&mut self.shared, wwin, widget);
                            let wid = window.window.id();

                            let action = window.init(&mut self.shared);
                            actions.push((wid, action));

                            self.id_map.insert(id, wid);
                            self.windows.insert(wid, window);
                        }
                        Err(e) => {
                            error!("Unable to create window: {}", e);
                        }
                    };
                }
                PendingAction::CloseWindow(id) => {
                    if let Some(id) = self.id_map.get(&id) {
                        actions.push((*id, TkAction::Close));
                    }
                }
            }
        }

        for (id, action) in actions.pop() {
            match action {
                TkAction::None => (),
                TkAction::Redraw => {
                    self.windows.get(&id).map(|w| w.window.request_redraw());
                }
                TkAction::Reconfigure => {
                    if let Some(window) = self.windows.get_mut(&id) {
                        self.resumes.retain(|resume| resume.1 != id);
                        if let Some(t) = window.reconfigure(&mut self.shared) {
                            self.resumes.push((t, id));
                            have_new_resumes = true;
                        }
                    }
                }
                TkAction::Close => {
                    if let Some(window) = self.windows.remove(&id) {
                        if window.handle_closure(&mut self.shared) == TkAction::CloseAll {
                            actions.push((id, TkAction::CloseAll));
                        }
                        // Wake immediately in order to evaluate pending actions:
                        *control_flow = ControlFlow::Poll;
                    }
                }
                TkAction::CloseAll => {
                    for (_id, window) in self.windows.drain() {
                        let _ = window.handle_closure(&mut self.shared);
                        // Pending actions are not evaluated; this is ok.
                    }
                    self.id_map.clear();
                    *control_flow = ControlFlow::Exit;
                }
            }
        }

        if have_new_resumes {
            self.resumes.sort_by_key(|item| item.0);

            *control_flow = if *control_flow == ControlFlow::Exit || self.windows.is_empty() {
                ControlFlow::Exit
            } else if *control_flow == ControlFlow::Poll {
                ControlFlow::Poll
            } else if let Some((instant, _)) = self.resumes.first() {
                trace!("Requesting resume at {:?}", *instant);
                ControlFlow::WaitUntil(*instant)
            } else {
                ControlFlow::Wait
            };
        }
    }
}
