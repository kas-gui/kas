// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event loop and handling

use log::{debug, error};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::time::Instant;

use winit::event::{Event, StartCause};
use winit::event_loop::{ControlFlow, EventLoopWindowTarget};
use winit::window as ww;

use kas::TkAction;
use kas_theme::Theme;

use crate::draw::{CustomPipe, DrawPipe};
use crate::shared::{PendingAction, SharedState};
use crate::{ProxyAction, Window, WindowId};

/// Event-loop data structure (i.e. all run-time state)
pub(crate) struct Loop<C: CustomPipe + 'static, T: Theme<DrawPipe<C>>>
where
    T::Window: kas_theme::Window,
{
    /// Window states
    windows: HashMap<ww::WindowId, Window<C::Window, T::Window>>,
    /// Translates our WindowId to winit's
    id_map: HashMap<WindowId, ww::WindowId>,
    /// Shared data passed from Toolkit
    shared: SharedState<C, T>,
    /// Timer resumes: (time, window index)
    resumes: Vec<(Instant, ww::WindowId)>,
}

impl<C: CustomPipe + 'static, T: Theme<DrawPipe<C>>> Loop<C, T>
where
    T::Window: kas_theme::Window,
{
    pub(crate) fn new(
        mut windows: Vec<Window<C::Window, T::Window>>,
        shared: SharedState<C, T>,
    ) -> Self {
        let id_map = windows
            .iter()
            .map(|w| (w.window_id, w.window.id()))
            .collect();
        Loop {
            windows: windows.drain(..).map(|w| (w.window.id(), w)).collect(),
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

        match event {
            WindowEvent { window_id, event } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.handle_event(&mut self.shared, event);
                }
            }

            DeviceEvent { .. } => return, // windows handle local input; we do not handle global input
            UserEvent(action) => match action {
                ProxyAction::Close(id) => {
                    if let Some(id) = self.id_map.get(&id) {
                        if let Some(window) = self.windows.get_mut(&id) {
                            window.send_action(TkAction::CLOSE);
                        }
                    }
                }
                ProxyAction::CloseAll => {
                    for window in self.windows.values_mut() {
                        window.send_action(TkAction::CLOSE);
                    }
                }
                ProxyAction::Update(handle, payload) => {
                    self.shared
                        .pending
                        .push(PendingAction::Update(handle, payload));
                }
            },

            NewEvents(cause) => {
                // MainEventsCleared will reset control_flow (but not when it is Poll)
                *control_flow = ControlFlow::Wait;

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
                        // debug!("Wakeup: WaitCancelled (ignoring)");
                    }
                    StartCause::Poll => (),
                    StartCause::Init => {
                        debug!("Wakeup: init");
                    }
                }
            }

            MainEventsCleared => {
                let mut close_all = false;
                let mut to_close = SmallVec::<[ww::WindowId; 4]>::new();
                for (window_id, window) in self.windows.iter_mut() {
                    let (action, resume) = window.update(&mut self.shared);
                    if action.contains(TkAction::EXIT) {
                        close_all = true;
                    } else if action.contains(TkAction::CLOSE) {
                        to_close.push(*window_id);
                    }
                    if let Some(instant) = resume {
                        if let Some((i, _)) = self
                            .resumes
                            .iter()
                            .enumerate()
                            .find(|item| (item.1).1 == *window_id)
                        {
                            self.resumes[i].0 = instant;
                        } else {
                            self.resumes.push((instant, *window_id));
                        }
                    }
                }

                for window_id in &to_close {
                    if let Some(window) = self.windows.remove(window_id) {
                        self.id_map.remove(&window.window_id);
                        if window
                            .handle_closure(&mut self.shared)
                            .contains(TkAction::EXIT)
                        {
                            close_all = true;
                        }
                        // Wake immediately in order to close remaining windows:
                        *control_flow = ControlFlow::Poll;
                    }
                }
                if close_all {
                    for (_, window) in self.windows.drain() {
                        let _ = window.handle_closure(&mut self.shared);
                    }
                }

                self.resumes.sort_by_key(|item| item.0);

                *control_flow = if *control_flow == ControlFlow::Exit || self.windows.is_empty() {
                    ControlFlow::Exit
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
                    window.do_draw(&mut self.shared);
                }
            }

            RedrawEventsCleared | LoopDestroyed | Suspended | Resumed => return,
        };

        // Create and init() any new windows.
        while let Some(pending) = self.shared.pending.pop() {
            match pending {
                PendingAction::AddPopup(parent_id, id, popup) => {
                    debug!("Adding overlay");
                    // TODO: support pop-ups as a special window, where available
                    self.windows.get_mut(&parent_id).unwrap().add_popup(
                        &mut self.shared,
                        id,
                        popup,
                    );
                    self.id_map.insert(id, parent_id);
                }
                PendingAction::AddWindow(id, widget) => {
                    debug!("Adding window {}", widget.title());
                    match Window::new(&mut self.shared, elwt, id, widget) {
                        Ok(window) => {
                            let wid = window.window.id();
                            self.id_map.insert(id, wid);
                            self.windows.insert(wid, window);
                        }
                        Err(e) => {
                            error!("Unable to create window: {}", e);
                        }
                    };
                }
                PendingAction::CloseWindow(id) => {
                    if let Some(wwid) = self.id_map.get(&id) {
                        if let Some(window) = self.windows.get_mut(&wwid) {
                            window.send_close(&mut self.shared, id);
                        }
                        self.id_map.remove(&id);
                    }
                }
                PendingAction::ThemeResize => {
                    for (_, window) in self.windows.iter_mut() {
                        window.theme_resize(&self.shared);
                    }
                }
                PendingAction::RedrawAll => {
                    for (_, window) in self.windows.iter_mut() {
                        window.window.request_redraw();
                    }
                }
                PendingAction::Update(handle, payload) => {
                    for window in self.windows.values_mut() {
                        window.update_handle(&mut self.shared, handle, payload);
                    }
                }
            }
        }
    }
}
