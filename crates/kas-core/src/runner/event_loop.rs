// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event loop and handling

use super::{AppData, GraphicsInstance, Pending, State};
use super::{ProxyAction, Window};
use crate::theme::Theme;
use crate::{Action, WindowId};
use std::collections::HashMap;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window as ww;

/// Event-loop data structure (i.e. all run-time state)
pub(super) struct Loop<A: AppData, G: GraphicsInstance, T: Theme<G::Shared>>
where
    T::Window: kas::theme::Window,
{
    /// State is suspended until we receive Event::Resumed
    suspended: bool,
    /// Window states
    windows: HashMap<WindowId, Box<Window<A, G, T>>>,
    popups: HashMap<WindowId, WindowId>,
    /// Translates our WindowId to winit's
    id_map: HashMap<ww::WindowId, WindowId>,
    /// Application state passed from Toolkit
    state: State<A, G, T>,
    /// Timer resumes: (time, window identifier)
    resumes: Vec<(Instant, WindowId)>,
}

impl<A: AppData, G, T> ApplicationHandler<ProxyAction> for Loop<A, G, T>
where
    G: GraphicsInstance,
    T: Theme<G::Shared>,
    T::Window: kas::theme::Window,
{
    fn new_events(&mut self, el: &ActiveEventLoop, cause: StartCause) {
        el.set_control_flow(ControlFlow::Wait);

        match cause {
            StartCause::ResumeTimeReached { .. } | StartCause::WaitCancelled { .. } => {
                let mut first_future = 0;
                for (i, resume) in self.resumes.iter().enumerate() {
                    if resume.0 > Instant::now() {
                        break;
                    }
                    first_future = i;

                    if let Some(w) = self.windows.get_mut(&resume.1) {
                        w.update_timer(&mut self.state, resume.0)
                    }
                }

                self.resumes.drain(..first_future);
            }
            StartCause::Poll => (),
            StartCause::Init => (),
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: ProxyAction) {
        match event {
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
                let mut stack = super::MessageStack::new();
                stack.push_erased(msg.into_erased());
                self.state.handle_messages(&mut stack);
            }
            ProxyAction::WakeAsync => {
                // We don't need to do anything here since about_to_wait will poll all futures.
            }
        }
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.suspended {
            for window in self.windows.values_mut() {
                match window.resume(&mut self.state, el) {
                    Ok(winit_id) => {
                        self.id_map.insert(winit_id, window.window_id());
                    }
                    Err(e) => {
                        log::error!("Unable to create window: {e}");
                    }
                }
            }
            self.suspended = false;
        }
    }

    fn window_event(
        &mut self,
        el: &ActiveEventLoop,
        window_id: ww::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(id) = self.id_map.get(&window_id)
            && let Some(window) = self.windows.get_mut(id)
            && window.handle_event(&mut self.state, event)
        {
            el.set_control_flow(ControlFlow::Poll);
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.flush_pending(el);
        self.resumes.sort_by_key(|item| item.0);

        if self.windows.is_empty() {
            el.exit();
        } else if matches!(el.control_flow(), ControlFlow::Poll) {
        } else if let Some((instant, _)) = self.resumes.first() {
            el.set_control_flow(ControlFlow::WaitUntil(*instant));
        } else {
            el.set_control_flow(ControlFlow::Wait);
        };
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        if !self.suspended {
            self.windows
                .retain(|_, window| window.suspend(&mut self.state));
            self.state.suspended();
            self.suspended = true;
        }
    }

    fn exiting(&mut self, el: &ActiveEventLoop) {
        self.suspended(el);
    }
}

impl<A: AppData, G: GraphicsInstance, T: Theme<G::Shared>> Loop<A, G, T>
where
    T::Window: kas::theme::Window,
{
    pub(super) fn new(mut windows: Vec<Box<Window<A, G, T>>>, state: State<A, G, T>) -> Self {
        Loop {
            suspended: true,
            windows: windows.drain(..).map(|w| (w.window_id(), w)).collect(),
            popups: Default::default(),
            id_map: Default::default(),
            state,
            resumes: vec![],
        }
    }

    fn flush_pending(&mut self, el: &ActiveEventLoop) {
        let mut close_all = false;
        while let Some(pending) = self.state.shared.pending.pop_front() {
            match pending {
                Pending::AddPopup(parent_id, id, popup) => {
                    log::debug!("Pending: adding overlay");
                    // TODO: support pop-ups as a special window, where available
                    self.windows
                        .get_mut(&parent_id)
                        .unwrap()
                        .add_popup(&mut self.state, id, popup);
                    self.popups.insert(id, parent_id);
                }
                Pending::AddWindow(id, mut window) => {
                    log::debug!("Pending: adding window {}", window.widget.title());
                    if !self.suspended {
                        match window.resume(&mut self.state, el) {
                            Ok(winit_id) => {
                                self.id_map.insert(winit_id, id);
                            }
                            Err(e) => {
                                log::error!("Unable to create window: {e}");
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
                        window.send_close(target);
                    }
                }
                Pending::Action(action) => {
                    if action.contains(Action::CLOSE) {
                        self.windows.clear();
                        self.id_map.clear();
                        el.set_control_flow(ControlFlow::Poll);
                    } else {
                        for (_, window) in self.windows.iter_mut() {
                            window.handle_action(&mut self.state, action);
                        }
                    }
                }
                Pending::Exit => close_all = true,
            }
        }

        self.resumes.clear();
        self.windows.retain(|window_id, window| {
            let (action, resume) = window.flush_pending(&mut self.state);
            if let Some(instant) = resume {
                self.resumes.push((instant, *window_id));
            }

            if close_all || action.contains(Action::CLOSE) {
                window.suspend(&mut self.state);

                // Call flush_pending again since suspend may queue messages.
                // We don't care about the returned Action or resume times since
                // the window is being destroyed.
                let _ = window.flush_pending(&mut self.state);

                self.id_map.retain(|_, v| v != window_id);
                false
            } else {
                true
            }
        });
    }
}
