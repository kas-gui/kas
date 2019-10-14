// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling

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

        NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::Wait;
        }
        NewEvents(_) => (), // we can ignore these events

        EventsCleared | LoopDestroyed | Suspended | Resumed => (),
    }
}
