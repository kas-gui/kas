// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shortcut matching

use super::{Command, ModifiersState, VirtualKeyCode};
use linear_map::LinearMap;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct Shortcuts {
    map: LinearMap<ModifiersState, HashMap<VirtualKeyCode, Command>>,
}

impl Shortcuts {
    /// Load default shortcuts
    pub fn load_defaults(&mut self) {
        use VirtualKeyCode as VK;
        #[cfg(target_os = "macos")]
        const CMD: ModifiersState = ModifiersState::LOGO;
        #[cfg(not(target_os = "macos"))]
        const CMD: ModifiersState = ModifiersState::CTRL;

        let map = self.map.entry(CMD).or_insert_with(Default::default);
        let shortcuts = [
            (VK::A, Command::SelectAll),
            (VK::C, Command::Copy),
            (VK::V, Command::Paste),
            (VK::X, Command::Cut),
            (VK::Z, Command::Undo),
        ];
        map.extend(shortcuts.iter().cloned());

        let modifiers = CMD | ModifiersState::SHIFT;
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        let shortcuts = [(VK::A, Command::Deselect), (VK::Z, Command::Redo)];
        map.extend(shortcuts.iter().cloned());
    }

    /// Match shortcuts
    pub fn get(&self, modifiers: ModifiersState, vkey: VirtualKeyCode) -> Option<Command> {
        self.map
            .get(&modifiers)
            .and_then(|m| m.get(&vkey))
            .cloned()
            .or_else(|| Command::new(vkey))
    }
}
