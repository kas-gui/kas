// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shortcut matching

use crate::event::{Command, Key, ModifiersState};
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
    map: LinearMap<ModifiersState, HashMap<Key, Command>>,
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
        #[cfg(target_os = "macos")]
        const CMD: ModifiersState = ModifiersState::SUPER;
        #[cfg(not(target_os = "macos"))]
        const CMD: ModifiersState = ModifiersState::CONTROL;

        // No modifiers
        #[cfg(not(target_os = "macos"))]
        {
            let modifiers = ModifiersState::empty();
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            let shortcuts = [
                (Key::F1, Command::Help),
                (Key::F2, Command::Rename),
                (Key::F3, Command::FindNext),
                (Key::F5, Command::Refresh),
                (Key::F7, Command::SpellCheck),
                (Key::F8, Command::Debug),
                (Key::F10, Command::Menu),
                (Key::F11, Command::Fullscreen),
            ];
            map.extend(shortcuts.iter().cloned());
        }

        // Shift
        #[cfg(not(target_os = "macos"))]
        {
            let modifiers = ModifiersState::SHIFT;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.insert(Key::F3, Command::FindPrevious);
        }

        // Alt (Option on MacOS)
        let modifiers = ModifiersState::ALT;
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        #[cfg(not(target_os = "macos"))]
        {
            let shortcuts = [
                (Key::F4, Command::Close),
                (Key::ArrowLeft, Command::NavPrevious),
                (Key::ArrowRight, Command::NavNext),
                (Key::ArrowUp, Command::NavParent),
                (Key::ArrowDown, Command::NavDown),
            ];
            map.extend(shortcuts.iter().cloned());
        }
        #[cfg(target_os = "macos")]
        {
            // Missing functionality: move to start/end of paragraph on (Shift)+Alt+Up/Down
            let shortcuts = [
                (Key::ArrowLeft, Command::WordLeft),
                (Key::ArrowRight, Command::WordRight),
            ];

            map.insert(Key::Delete, Command::DelWordBack);
            map.extend(shortcuts.iter().cloned());

            // Shift + Option
            let modifiers = ModifiersState::SHIFT | ModifiersState::ALT;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.extend(shortcuts.iter().cloned());
        }

        // Command (MacOS) or Ctrl (other OS)
        let map = self.map.entry(CMD).or_insert_with(Default::default);
        let shortcuts = [
            (Key::Character("a".into()), Command::SelectAll),
            (Key::Character("b".into()), Command::Bold),
            (Key::Character("c".into()), Command::Copy),
            (Key::Character("f".into()), Command::Find),
            (Key::Character("i".into()), Command::Italic),
            (Key::Character("k".into()), Command::Link),
            (Key::Character("n".into()), Command::New),
            (Key::Character("o".into()), Command::Open),
            (Key::Character("p".into()), Command::Print),
            (Key::Character("s".into()), Command::Save),
            (Key::Character("t".into()), Command::TabNew),
            (Key::Character("u".into()), Command::Underline),
            (Key::Character("v".into()), Command::Paste),
            (Key::Character("]".into()), Command::Paste),
            (Key::Character("w".into()), Command::Close),
            (Key::Character("x".into()), Command::Cut),
            (Key::Character("z".into()), Command::Undo),
            (Key::Tab, Command::TabNext),
        ];
        map.extend(shortcuts.iter().cloned());
        #[cfg(target_os = "macos")]
        {
            let shortcuts = [
                (Key::Character("g".into()), Command::FindNext),
                (Key::ArrowUp, Command::DocHome),
                (Key::ArrowDown, Command::DocEnd),
                (Key::ArrowLeft, Command::Home),
                (Key::ArrowRight, Command::End),
            ];
            map.extend(shortcuts.iter().cloned());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let shortcuts = [
                (Key::Character("q".into()), Command::Exit),
                (Key::Character("r".into()), Command::FindReplace),
            ];
            map.extend(shortcuts.iter().cloned());

            let shortcuts = [
                (Key::ArrowUp, Command::ViewUp),
                (Key::ArrowDown, Command::ViewDown),
                (Key::ArrowLeft, Command::WordLeft),
                (Key::ArrowRight, Command::WordRight),
                (Key::Backspace, Command::DelWordBack),
                (Key::Delete, Command::DelWord),
                (Key::Home, Command::DocHome),
                (Key::End, Command::DocEnd),
                (Key::PageUp, Command::TabPrevious),
                (Key::PageDown, Command::TabNext),
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
            let modifiers = ModifiersState::CONTROL | ModifiersState::SUPER;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.insert(Key::Character("f".into()), Command::Fullscreen);
        }

        // Shift + Ctrl/Command
        let modifiers = ModifiersState::SHIFT | CMD;
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        let shortcuts = [
            (Key::Character("a".into()), Command::Deselect),
            (Key::Character("z".into()), Command::Redo),
            (Key::Tab, Command::TabPrevious),
        ];
        map.extend(shortcuts.iter().cloned());
        #[cfg(target_os = "macos")]
        {
            let shortcuts = [
                (Key::Character("g".into()), Command::FindPrevious),
                (Key::Character(":".into()), Command::SpellCheck),
                (Key::ArrowUp, Command::DocHome),
                (Key::ArrowDown, Command::DocEnd),
                (Key::ArrowLeft, Command::Home),
                (Key::ArrowRight, Command::End),
            ];
            map.extend(shortcuts.iter().cloned());
        }

        // Alt + Command (MacOS)
        #[cfg(target_os = "macos")]
        {
            let modifiers = ModifiersState::ALT | CMD;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.insert(Key::Character("w".into()), Command::Exit);
        }
    }

    /// Match shortcuts
    ///
    /// Note: text-editor navigation keys (e.g. arrows, home/end) result in the
    /// same output with and without Shift pressed. Editors should check the
    /// status of the Shift modifier directly where this has an affect.
    pub fn try_match(&self, mut modifiers: ModifiersState, key: &Key) -> Option<Command> {
        if let Some(result) = self.map.get(&modifiers).and_then(|m| m.get(key)) {
            return Some(*result);
        }
        modifiers.remove(ModifiersState::SHIFT);
        if modifiers.is_empty() {
            // These keys get matched with and without Shift:
            return Command::new(key);
        }
        None
    }
}

#[cfg(feature = "serde")]
fn state_to_string(state: ModifiersState) -> &'static str {
    const SHIFT: ModifiersState = ModifiersState::SHIFT;
    const CONTROL: ModifiersState = ModifiersState::CONTROL;
    const ALT: ModifiersState = ModifiersState::ALT;
    const SUPER: ModifiersState = ModifiersState::SUPER;
    // we can't use match since OR patterns are unstable (rust#54883)
    if state == ModifiersState::empty() {
        "none"
    } else if state == SUPER {
        "super"
    } else if state == ALT {
        "alt"
    } else if state == ALT | SUPER {
        "alt-super"
    } else if state == CONTROL {
        "ctrl"
    } else if state == CONTROL | SUPER {
        "ctrl-super"
    } else if state == CONTROL | ALT {
        "ctrl-alt"
    } else if state == CONTROL | ALT | SUPER {
        "ctrl-alt-super"
    } else if state == SHIFT {
        "shift"
    } else if state == SHIFT | SUPER {
        "shift-super"
    } else if state == SHIFT | ALT {
        "alt-shift"
    } else if state == SHIFT | ALT | SUPER {
        "alt-shift-super"
    } else if state == SHIFT | CONTROL {
        "ctrl-shift"
    } else if state == SHIFT | CONTROL | SUPER {
        "ctrl-shift-super"
    } else if state == SHIFT | CONTROL | ALT {
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
            // TODO: winit::keyboard::Key does not support Ord!
            // let bindings: std::collections::BTreeMap<_, _> = bindings.iter().collect();
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
            state |= ModifiersState::CONTROL;
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
            state |= ModifiersState::SUPER;
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
        let mut map = LinearMap::<ModifiersState, HashMap<Key, Command>>::new();
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
