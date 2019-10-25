// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling

use std::time::Instant;

use winit::event::{Event, StartCause};
use winit::event_loop::ControlFlow;

use crate::Window;

#[inline]
pub(crate) fn handler<T>(
    windows: &mut Vec<Window>,
    event: Event<T>,
    control_flow: &mut ControlFlow,
) {
    use Event::*;
    match event {
        WindowEvent { window_id, event } => {
            let mut to_close = None;
            for (i, window) in windows.iter_mut().enumerate() {
                if window.ww.id() == window_id {
                    if window.handle_event(event) {
                        to_close = Some(i);
                    }
                    break;
                }
            }
            if let Some(i) = to_close {
                windows.remove(i);
                if windows.is_empty() {
                    *control_flow = ControlFlow::Exit;
                }
            }
        }

        DeviceEvent { .. } => (), // windows handle local input; we do not handle global input
        UserEvent(_) => (),       // we have no handler for user events

        NewEvents(cause) => {
            match cause {
                StartCause::ResumeTimeReached {
                    requested_resume, ..
                } => {
                    for window in windows.iter_mut() {
                        window.timer_resume(requested_resume);
                    }
                    *control_flow = next_resume_time(windows);
                }
                StartCause::Init => *control_flow = next_resume_time(windows),
                _ => (), // we can ignore these events
            }
        }

        EventsCleared | LoopDestroyed | Suspended | Resumed => (),
    }
}

fn next_resume_time(windows: &mut Vec<Window>) -> ControlFlow {
    let mut resume_time: Option<Instant> = None;
    for window in windows.iter() {
        if let Some(time) = window.next_resume() {
            resume_time = match resume_time {
                Some(t) => Some(t.min(time)),
                None => Some(time),
            };
        }
    }
    resume_time
        .map(|t| ControlFlow::WaitUntil(t))
        .unwrap_or(ControlFlow::Wait)
}
