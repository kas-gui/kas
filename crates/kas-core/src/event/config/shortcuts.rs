// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shortcut matching

use crate::event::{Command, ModifiersState, VirtualKeyCode};
use linear_map::LinearMap;
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, MapAccess, Unexpected, Visitor};
#[cfg(feature = "serde")]
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::HashMap;
#[cfg(feature = "serde")] use std::fmt;

/// Shortcut manager
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, PartialEq)]
pub struct Shortcuts {
    map: LinearMap<ModifiersState, HashMap<VirtualKeyCode, Command>>,
}

impl Shortcuts {
    /// Construct, with no bindings
    #[inline]
    pub fn empty() -> Self {
        Shortcuts {
            map: Default::default(),
        }
    }

    /// Construct, with default bindings
    #[inline]
    pub fn platform_defaults() -> Self {
        let mut s = Self::empty();
        s.load_platform_defaults();
        s
    }

    /// Load default shortcuts for the current platform
    pub fn load_platform_defaults(&mut self) {
        use VirtualKeyCode as VK;
        #[cfg(target_os = "macos")]
        const CMD: ModifiersState = ModifiersState::LOGO;
        #[cfg(not(target_os = "macos"))]
        const CMD: ModifiersState = ModifiersState::CTRL;

        // No modifiers
        #[cfg(not(target_os = "macos"))]
        {
            let modifiers = ModifiersState::empty();
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            let shortcuts = [
                (VK::F1, Command::Help),
                (VK::F2, Command::Rename),
                (VK::F3, Command::FindNext),
                (VK::F5, Command::Refresh),
                (VK::F7, Command::Spelling),
                (VK::F8, Command::Debug),
                (VK::F10, Command::Menu),
                (VK::F11, Command::Fullscreen),
            ];
            map.extend(shortcuts.iter().cloned());
        }

        // Shift
        #[cfg(not(target_os = "macos"))]
        {
            let modifiers = ModifiersState::SHIFT;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
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
                (VK::Left, Command::WordLeft),
                (VK::Right, Command::WordRight),
            ];

            map.insert(VK::Delete, Command::DelWordBack);
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
                (VK::Up, Command::DocHome),
                (VK::Down, Command::DocEnd),
                (VK::Left, Command::Home),
                (VK::Right, Command::End),
            ];
            map.extend(shortcuts.iter().cloned());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let shortcuts = [(VK::Q, Command::Exit), (VK::R, Command::FindReplace)];
            map.extend(shortcuts.iter().cloned());

            let shortcuts = [
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

            // Shift + Ctrl
            let modifiers = ModifiersState::SHIFT | CMD;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
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
                (VK::G, Command::FindPrev),
                (VK::Colon, Command::Spelling),
                (VK::Up, Command::DocHome),
                (VK::Down, Command::DocEnd),
                (VK::Left, Command::Home),
                (VK::Right, Command::End),
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
    ///
    /// Note: text-editor navigation keys (e.g. arrows, home/end) result in the
    /// same output with and without Shift pressed. Editors should check the
    /// status of the Shift modifier directly where this has an affect.
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

#[cfg(feature = "serde")]
fn state_to_string(state: ModifiersState) -> &'static str {
    const SHIFT: ModifiersState = ModifiersState::SHIFT;
    const CTRL: ModifiersState = ModifiersState::CTRL;
    const ALT: ModifiersState = ModifiersState::ALT;
    const SUPER: ModifiersState = ModifiersState::LOGO;
    // we can't use match since OR patterns are unstable (rust#54883)
    if state == ModifiersState::empty() {
        "none"
    } else if state == SUPER {
        "super"
    } else if state == ALT {
        "alt"
    } else if state == ALT | SUPER {
        "alt-super"
    } else if state == CTRL {
        "ctrl"
    } else if state == CTRL | SUPER {
        "ctrl-super"
    } else if state == CTRL | ALT {
        "ctrl-alt"
    } else if state == CTRL | ALT | SUPER {
        "ctrl-alt-super"
    } else if state == SHIFT {
        "shift"
    } else if state == SHIFT | SUPER {
        "shift-super"
    } else if state == SHIFT | ALT {
        "alt-shift"
    } else if state == SHIFT | ALT | SUPER {
        "alt-shift-super"
    } else if state == SHIFT | CTRL {
        "ctrl-shift"
    } else if state == SHIFT | CTRL | SUPER {
        "ctrl-shift-super"
    } else if state == SHIFT | CTRL | ALT {
        "ctrl-alt-shift"
    } else {
        "ctrl-alt-shift-super"
    }
}

#[cfg(feature = "serde")]
impl Serialize for Shortcuts {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = s.serialize_map(Some(self.map.len()))?;
        for (state, bindings) in &self.map {
            map.serialize_key(state_to_string(*state))?;

            // Sort items in the hash-map to ensure stable order
            // NOTE: We need a "map type" to ensure entries are serialised as
            // a map, not as a list. BTreeMap is easier than a shim over a Vec.
            let bindings: std::collections::BTreeMap<_, _> = bindings.iter().collect();
            map.serialize_value(&bindings)?;
        }
        map.end()
    }
}

// #[derive(Error, Debug)]
// pub enum DeError {
//     #[error("invalid modifier state: {0}")]
//     State(String),
// }

#[cfg(feature = "serde")]
struct ModifierStateVisitor(ModifiersState);
#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for ModifierStateVisitor {
    type Value = ModifierStateVisitor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("none or ctrl or alt-shift-super etc.")
    }

    fn visit_str<E: de::Error>(self, u: &str) -> Result<Self::Value, E> {
        let mut v = u;
        let mut state = ModifiersState::empty();

        if v.starts_with("ctrl") {
            state |= ModifiersState::CTRL;
            v = &v[v.len().min(4)..];
        }
        if v.starts_with('-') {
            v = &v[1..];
        }
        if v.starts_with("alt") {
            state |= ModifiersState::ALT;
            v = &v[v.len().min(3)..];
        }
        if v.starts_with('-') {
            v = &v[1..];
        }
        if v.starts_with("shift") {
            state |= ModifiersState::SHIFT;
            v = &v[v.len().min(5)..];
        }
        if v.starts_with('-') {
            v = &v[1..];
        }
        if v.starts_with("super") {
            state |= ModifiersState::LOGO;
            v = &v[v.len().min(5)..];
        }

        if v.is_empty() || u == "none" {
            Ok(ModifierStateVisitor(state))
        } else {
            Err(E::invalid_value(
                Unexpected::Str(u),
                &"none or ctrl or alt-shift-super etc.",
            ))
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ModifierStateVisitor {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_str(ModifierStateVisitor(Default::default()))
    }
}

#[cfg(feature = "serde")]
struct ShortcutsVisitor;
#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for ShortcutsVisitor {
    type Value = Shortcuts;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("{ <modifiers> : { <key> : <command> } }")
    }

    fn visit_map<A>(self, mut reader: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut map = LinearMap::<ModifiersState, HashMap<VirtualKeyCode, Command>>::new();
        while let Some(key) = reader.next_key::<ModifierStateVisitor>()? {
            let value = reader.next_value()?;
            map.insert(key.0, value);
        }
        Ok(Shortcuts { map })
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Shortcuts {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_map(ShortcutsVisitor)
    }
}
