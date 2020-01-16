// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event loop and handling

use log::{debug, error, trace};
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
        let (id, action) = match event {
            WindowEvent { window_id, event } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    (window_id, window.handle_event(&mut self.shared, event))
                } else {
                    return;
                }
            }

            DeviceEvent { .. } => return, // windows handle local input; we do not handle global input
            UserEvent(action) => match action {
                ProxyAction::Close(id) => {
                    if let Some(id) = self.id_map.get(&id) {
                        (*id, TkAction::Close)
                    } else {
                        return; // window already closed
                    }
                }
                ProxyAction::CloseAll => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            },

            NewEvents(cause) => {
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

                        let (action, resume) = if let Some(w) = self.windows.get_mut(&item.1) {
                            w.timer_resume(&mut self.shared, requested_resume)
                        } else {
                            // presumably, some window with active timers was removed
                            self.resumes.remove(0);
                            return;
                        };
                        if let Some(instant) = resume {
                            self.resumes[0].0 = instant;
                            self.resumes.sort_by_key(|item| item.0);
                            trace!("Requesting resume at {:?}", self.resumes[0].0);
                            *control_flow = ControlFlow::WaitUntil(self.resumes[0].0);
                        } else {
                            self.resumes.remove(0);
                        }

                        (item.1, action)
                    }

                    StartCause::Init => {
                        debug!("Wakeup: init");

                        for (id, window) in self.windows.iter_mut() {
                            if let Some(instant) = window.init(&mut self.shared) {
                                self.resumes.push((instant, *id));
                            }
                        }
                        self.resumes.sort_by_key(|item| item.0);
                        if let Some(first) = self.resumes.first() {
                            trace!("Requesting resume at {:?}", first.0);
                            *control_flow = ControlFlow::WaitUntil(first.0);
                        } else {
                            *control_flow = ControlFlow::Wait;
                        }
                        return;
                    }
                    _ => return, // we can ignore these events
                }
            }

            EventsCleared | LoopDestroyed | Suspended | Resumed => return,
        };

        // Create and init() any new windows.
        let mut have_new_resumes = false;
        while let Some(pending) = self.shared.pending.pop() {
            match pending {
                PendingAction::AddWindow(id, widget) => {
                    debug!("Adding window {}", widget.title());
                    match winit::window::Window::new(elwt) {
                        Ok(window) => {
                            window.set_title(widget.title());
                            let mut win = Window::new(&mut self.shared, window, widget);
                            let wid = win.window.id();
                            if let Some(instant) = win.init(&mut self.shared) {
                                self.resumes.push((instant, wid));
                                have_new_resumes = true;
                            }
                            self.id_map.insert(id, wid);
                            self.windows.insert(wid, win);
                        }
                        Err(e) => {
                            error!("Unable to create window: {}", e);
                        }
                    };
                }
            }
        }
        if have_new_resumes {
            self.resumes.sort_by_key(|item| item.0);
            if let Some(first) = self.resumes.first() {
                trace!("Requesting resume at {:?}", first.0);
                *control_flow = ControlFlow::WaitUntil(first.0);
            } else {
                *control_flow = ControlFlow::Wait;
            }
        }

        match action {
            TkAction::None => (),
            TkAction::Redraw => {
                self.windows.get(&id).map(|w| w.window.request_redraw());
            }
            TkAction::Reconfigure => {
                self.windows.get_mut(&id).map(|w| w.reconfigure());
            }
            TkAction::Close => {
                self.windows.remove(&id);
                if self.windows.is_empty() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            TkAction::CloseAll => *control_flow = ControlFlow::Exit,
        }
    }
}
