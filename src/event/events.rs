// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling
//!
//! Event handling uses *event* messages, passed from the parent into a widget,
//! with responses passed back to the parent. This model is simpler than that
//! commonly used by GUI frameworks: widgets do not need a pointer to their
//! parent and any result is pushed back up the call stack. The model allows
//! type-safety while allowing user-defined result types.

use super::{ElementState, ModifiersState, MouseButton};

use crate::geom::Coord;
use crate::WidgetId;

/// High-level actions supported by widgets
#[derive(Debug)]
pub enum Action {
    /// Widget activation, for example clicking a button or toggling a check-box
    Activate,
    Dummy, // exists temporarily to allow _ pattern in matchers
}

/// Input events: these are low-level messages where the destination widget is
/// unknown.
///
/// These events are segregated by delivery method.
#[derive(Debug)]
pub enum Event {
    /* NOTE: it's tempting to add this, but we have no model for returning a
     * response from multiple recipients and no use-case.
    /// Events to be addressed to all descendents
    ToAll(EventAll),
    */
    /// Events addressed to a child by [`WidgetId`]
    ToChild(WidgetId, EventChild),
    /// Events addressed by coordinate
    ToCoord(Coord, EventCoord),
}

/// Events addressed to a child by [`WidgetId`]
#[derive(Debug)]
pub enum EventChild {
    MouseInput {
        state: ElementState,
        button: MouseButton,
        modifiers: ModifiersState,
    },
}

/// Events addressed by coordinate
#[derive(Debug)]
pub enum EventCoord {
    CursorMoved { modifiers: ModifiersState },
    TouchStart(u64),
    TouchMove(u64),
    TouchEnd(u64),
}

// TODO:
//     DroppedFile(PathBuf),
//     HoveredFile(PathBuf),
//     HoveredFileCancelled,
//     ReceivedCharacter(char),
//     Focused(bool),
//     KeyboardInput {
//         device_id: DeviceId,
//         input: KeyboardInput,
//     },
//     CursorEntered {
//         device_id: DeviceId,
//     },
//     CursorLeft {
//         device_id: DeviceId,
//     },
//     MouseWheel {
//         device_id: DeviceId,
//         delta: MouseScrollDelta,
//         phase: TouchPhase,
//         modifiers: ModifiersState,
//     },
//     TouchpadPressure {
//         device_id: DeviceId,
//         pressure: f32,
//         stage: i64,
//     },
//     AxisMotion {
//         device_id: DeviceId,
//         axis: AxisId,
//         value: f64,
//     },
