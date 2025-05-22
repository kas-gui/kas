// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event loop and handling

use super::{AppData, GraphicsInstance, Pending, Shared};
use super::{ProxyAction, Window};
use crate::theme::Theme;
use crate::{WindowAction, window::WindowId};
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
    /// Shared application state
    shared: Shared<A, G, T>,
    /// User-provided application data
    data: A,
    /// Timer resumes: (time, window identifier)
    resumes: Vec<(Instant, WindowId)>,
}

impl<A: AppData, G, T> ApplicationHandler for Loop<A, G, T>
where
    G: GraphicsInstance,
    T: Theme<G::Shared>,
    T::Window: kas::theme::Window,
{
    fn new_events(&mut self, el: &dyn ActiveEventLoop, cause: StartCause) {
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
                        w.update_timer(&mut self.shared, &self.data, resume.0)
                    }
                }

                self.resumes.drain(..first_future);
            }
            StartCause::Poll => (),
            StartCause::Init => (),
        }
    }

    fn proxy_wake_up(&mut self, _: &dyn ActiveEventLoop) {
        while let Ok(event) = self.shared.proxy_rx.try_recv() {
            match event {
                ProxyAction::Close(id) => {
                    if let Some(window) = self.windows.get_mut(&id) {
                        window.send_close(id);
                    }
                }
                ProxyAction::CloseAll => {
                    for (id, window) in self.windows.iter_mut() {
                        window.send_close(*id);
                    }
                }
                ProxyAction::Message(msg) => {
                    // Message is pushed in self.about_to_wait()
                    self.shared.messages.push_erased(msg.into_erased());
                }
                #[cfg(feature = "accesskit")]
                ProxyAction::AccessKit(window_id, event) => {
                    if let Some(id) = self.id_map.get(&window_id)
                        && let Some(window) = self.windows.get_mut(id)
                    {
                        window.accesskit_event(&mut self.shared, &self.data, event);
                    }
                }
            }
        }
    }

    fn resumed(&mut self, _: &dyn ActiveEventLoop) {
        if self.suspended {
            self.data.resumed();
            self.suspended = false;
        }
    }

    fn can_create_surfaces(&mut self, el: &dyn ActiveEventLoop) {
        if self.suspended {
            self.resumed(el);
        }

        for window in self.windows.values_mut() {
            match window.create_surfaces(&mut self.shared, &self.data, el, None) {
                Ok(winit_id) => {
                    self.id_map.insert(winit_id, window.window_id());
                }
                Err(e) => {
                    log::error!("Unable to create window: {e}");
                }
            }
        }
    }

    fn window_event(
        &mut self,
        el: &dyn ActiveEventLoop,
        window_id: ww::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(id) = self.id_map.get(&window_id)
            && let Some(window) = self.windows.get_mut(id)
            && window.handle_event(&mut self.shared, &self.data, event)
        {
            el.set_control_flow(ControlFlow::Poll);
        }
    }

    fn about_to_wait(&mut self, el: &dyn ActiveEventLoop) {
        self.flush_pending(el);
        self.shared.handle_messages(&mut self.data);

        // Distribute inter-window messages.
        // NOTE: sending of these messages will be delayed until the next call to flush_pending.
        while let Some((id, msg)) = self.shared.send_queue.pop_front() {
            if let Some(mut window_id) = id.window_id() {
                if let Some(win_id) = self.popups.get(&window_id) {
                    window_id = *win_id;
                }
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.send_erased(id, msg);
                }
            } else {
                log::warn!("unable to send message (no target): {msg:?}");
            }
        }

        self.resumes.sort_by_key(|item| item.0);

        if self.windows.is_empty() {
            if !self.suspended {
                self.destroy_surfaces(el);
            }
            el.exit();
        } else if matches!(el.control_flow(), ControlFlow::Poll) {
        } else if let Some((instant, _)) = self.resumes.first() {
            el.set_control_flow(ControlFlow::WaitUntil(*instant));
        } else {
            el.set_control_flow(ControlFlow::Wait);
        };
    }

    fn suspended(&mut self, _: &dyn ActiveEventLoop) {
        if !self.suspended {
            self.windows
                .retain(|_, window| window.suspend(&mut self.shared, &self.data));
            self.data.suspended();
            self.shared.suspended();
            self.suspended = true;
        }
    }

    fn destroy_surfaces(&mut self, el: &dyn ActiveEventLoop) {
        if !self.suspended {
            self.suspended(el);
        }

        for window in self.windows.values_mut() {
            window.destroy_surfaces();
        }
    }

    fn memory_warning(&mut self, _: &dyn ActiveEventLoop) {
        self.data.memory_warning();
    }
}

impl<A: AppData, G: GraphicsInstance, T: Theme<G::Shared>> Loop<A, G, T>
where
    T::Window: kas::theme::Window,
{
    pub(super) fn new(
        mut windows: Vec<Box<Window<A, G, T>>>,
        shared: Shared<A, G, T>,
        data: A,
    ) -> Self {
        Loop {
            suspended: true,
            windows: windows.drain(..).map(|w| (w.window_id(), w)).collect(),
            popups: Default::default(),
            id_map: Default::default(),
            shared,
            data,
            resumes: vec![],
        }
    }

    fn flush_pending(&mut self, el: &dyn ActiveEventLoop) {
        let mut close_all = false;
        while let Some(pending) = self.shared.pending.pop_front() {
            match pending {
                Pending::Update => {
                    for (_, window) in self.windows.iter_mut() {
                        window.update(&self.data);
                    }
                }
                Pending::ConfigUpdate(action) => {
                    for (_, window) in self.windows.iter_mut() {
                        window.config_update(&mut self.shared, &self.data, action);
                    }
                }
                Pending::AddPopup(parent_id, id, popup) => {
                    log::debug!("Pending: adding overlay");
                    // TODO: support pop-ups as a special window, where available
                    self.windows
                        .get_mut(&parent_id)
                        .unwrap()
                        .add_popup(&self.data, id, popup);
                    self.popups.insert(id, parent_id);
                }
                Pending::RepositionPopup(id, popup) => {
                    if let Some(parent_id) = self.popups.get(&id) {
                        self.windows
                            .get_mut(parent_id)
                            .unwrap()
                            .add_popup(&self.data, id, popup);
                    }
                }
                Pending::AddWindow(id, window) => {
                    let mut window = Box::new(Window::new(
                        self.shared.config.clone(),
                        self.shared.platform,
                        id,
                        window,
                    ));

                    log::debug!("Pending: adding window {}", window.widget.title());
                    if !self.suspended {
                        let mut modal_parent = None;
                        if let Some(id) = window.widget.properties().modal_parent
                            && let Some(window) = self.windows.get(&id)
                        {
                            modal_parent = window.winit_window();
                        }
                        match window.create_surfaces(&mut self.shared, &self.data, el, modal_parent)
                        {
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
                        window.send_close(&mut self.state, target);
                    }
                }
                Pending::Exit => close_all = true,
            }
        }

        self.resumes.clear();
        self.windows.retain(|window_id, window| {
            let (action, resume) = window.flush_pending(&mut self.shared, &self.data);
            if let Some(instant) = resume {
                self.resumes.push((instant, *window_id));
            }

            if close_all || action.contains(WindowAction::CLOSE) {
                window.suspend(&mut self.shared, &self.data);

                // Call flush_pending again since suspend may queue messages.
                // We don't care about the returned WindowAction or resume times since
                // the window is being destroyed.
                let _ = window.flush_pending(&mut self.shared, &self.data);

                self.id_map.retain(|_, v| v != window_id);
                false
            } else {
                true
            }
        });
    }
}
