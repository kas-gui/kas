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
    ///
    /// Note: text-editor move keys are repeated with shift so that e.g.
    /// Shift+Home is matched. Such actions do not have unique names; the
    /// consumer must check the status of the shift modifier directly.
    pub fn load_defaults(&mut self) {
        use VirtualKeyCode as VK;
        #[cfg(target_os = "macos")]
        const CMD: ModifiersState = ModifiersState::LOGO;
        #[cfg(not(target_os = "macos"))]
        const CMD: ModifiersState = ModifiersState::CTRL;

        // No modifiers
        let modifiers = ModifiersState::empty();
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        #[cfg(not(target_os = "macos"))]
        {
            let shortcuts = [
                (VK::F1, Command::Help),
                (VK::F2, Command::Rename),
                (VK::F3, Command::FindNext),
                (VK::F5, Command::Refresh),
                (VK::F7, Command::Spelling),
                (VK::F10, Command::Menu),
                (VK::F11, Command::Fullscreen),
            ];
            map.extend(shortcuts.iter().cloned());
        }

        // Shift
        let modifiers = ModifiersState::SHIFT;
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        #[cfg(not(target_os = "macos"))]
        {
            map.insert(VK::F3, Command::FindPrev);
        }

        // Alt (Option on MacOS)
        let modifiers = ModifiersState::ALT;
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        #[cfg(not(target_os = "macos"))]
        {
            let shortcuts = [
                (VK::F4, Command::Close),
                (VK::Left, Command::NavPrev),
                (VK::Right, Command::NavNext),
                (VK::Up, Command::NavParent),
                (VK::Down, Command::NavDown),
            ];
            map.extend(shortcuts.iter().cloned());
        }
        #[cfg(target_os = "macos")]
        {
            // Missing functionality: move to start/end of paragraph on (Shift)+Alt+Up/Down
            let shortcuts = [
                (VK::Left, Command::MovePrevWord),
                (VK::Right, Command::MoveNextWord),
            ];

            map.insert(VK::Delete, Command::DelPrevWord);
            map.extend(shortcuts.iter().cloned());

            // Shift + Option
            let modifiers = ModifiersState::SHIFT | ModifiersState::ALT;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.extend(shortcuts.iter().cloned());
        }

        // Command (MacOS) or Ctrl (other OS)
        let map = self.map.entry(CMD).or_insert_with(Default::default);
        let shortcuts = [
            (VK::A, Command::SelectAll),
            (VK::B, Command::Bold),
            (VK::C, Command::Copy),
            (VK::F, Command::Find),
            (VK::I, Command::Italic),
            (VK::K, Command::Link),
            (VK::N, Command::New),
            (VK::O, Command::Open),
            (VK::P, Command::Print),
            (VK::S, Command::Save),
            (VK::T, Command::TabNew),
            (VK::U, Command::Underline),
            (VK::V, Command::Paste),
            (VK::W, Command::Close),
            (VK::X, Command::Cut),
            (VK::Z, Command::Undo),
            (VK::Tab, Command::TabNext),
        ];
        map.extend(shortcuts.iter().cloned());
        #[cfg(target_os = "macos")]
        {
            let shortcuts = [
                (VK::G, Command::FindNext),
                (VK::Up, Command::DocStart),
                (VK::Down, Command::DocEnd),
                (VK::Left, Command::LineStart),
                (VK::Right, Command::LineEnd),
            ];
            map.extend(shortcuts.iter().cloned());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let shortcuts = [
                (VK::Q, Command::Exit),
                (VK::R, Command::FindReplace),
                (VK::Up, Command::ViewUp),
                (VK::Down, Command::ViewDown),
                (VK::Left, Command::WordLeft),
                (VK::Right, Command::WordRight),
                (VK::Back, Command::DelWordBack),
                (VK::Delete, Command::DelWord),
                (VK::Home, Command::DocHome),
                (VK::End, Command::DocEnd),
                (VK::PageUp, Command::TabPrev),
                (VK::PageDown, Command::TabNext),
            ];
            map.extend(shortcuts.iter().cloned());
        }

        // Ctrl + Command (MacOS)
        #[cfg(target_os = "macos")]
        {
            let modifiers = ModifiersState::CTRL | ModifiersState::LOGO;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.insert(VK::F, Command::Fullscreen);
        }

        // Shift + Ctrl/Command
        let modifiers = ModifiersState::SHIFT | CMD;
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        let shortcuts = [
            (VK::A, Command::Deselect),
            (VK::Z, Command::Redo),
            (VK::Tab, Command::TabPrev),
        ];
        map.extend(shortcuts.iter().cloned());
        #[cfg(target_os = "macos")]
        {
            let shortcuts = [
                (VK::G, Command::FindPrevious),
                (VK::Colon, Command::Spelling),
                (VK::Up, Command::DocStart),
                (VK::Down, Command::DocEnd),
                (VK::Left, Command::LineStart),
                (VK::Right, Command::LineEnd),
            ];
            map.extend(shortcuts.iter().cloned());
        }

        // Alt + Command (MacOS)
        #[cfg(target_os = "macos")]
        {
            let modifiers = ModifiersState::ALT | CMD;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.insert(VK::W, Command::Exit);
        }
    }

    /// Match shortcuts
    pub fn get(&self, mut modifiers: ModifiersState, vkey: VirtualKeyCode) -> Option<Command> {
        if let Some(result) = self.map.get(&modifiers).and_then(|m| m.get(&vkey)) {
            return Some(*result);
        }
        modifiers.remove(ModifiersState::SHIFT);
        if modifiers.is_empty() {
            // These keys get matched with and without Shift:
            return Command::new(vkey);
        }
        None
    }
}
