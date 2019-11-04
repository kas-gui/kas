// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event enum fallbacks for use without winit

/// Identifier of an input device.
///
/// When compiled without the `winit` feature, this is just a dummy type.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct DeviceId;

impl DeviceId {
    /// Returns a dummy `DeviceId`. The only guarantees made about the return
    /// value of this function is that it will always be equal to itself and to
    /// future values returned by this function.
    ///
    /// In contrast to winit's equivalent, this function is safe. If KAS's winit
    /// dependency is enabled, calls to this function will become unsafe.
    pub fn dummy() -> Self {
        DeviceId
    }
}

/// Describes the input state of a key.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ElementState {
    Pressed,
    Released,
}

/// Represents the current state of the keyboard modifiers
///
/// Each field of this struct represents a modifier and is `true` if this modifier is active.
#[derive(Default, Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ModifiersState {
    /// The "shift" key
    pub shift: bool,
    /// The "control" key
    pub ctrl: bool,
    /// The "alt" key
    pub alt: bool,
    /// The "logo" key
    ///
    /// This is the "windows" key on PC and "command" key on Mac.
    pub logo: bool,
}

/// Describes a button of a mouse controller.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}
