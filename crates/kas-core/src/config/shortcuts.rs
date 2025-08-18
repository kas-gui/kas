// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shortcut matching

use crate::event::{Command, Key, ModifiersState};
use linear_map::LinearMap;
use std::collections::HashMap;
use winit::keyboard::NamedKey;

/// Shortcut manager
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, PartialEq)]
pub struct Shortcuts {
    // NOTE: we do not permit Key::Dead(None) here
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
                (NamedKey::F1.into(), Command::Help),
                (NamedKey::F2.into(), Command::Rename),
                (NamedKey::F3.into(), Command::FindNext),
                (NamedKey::F5.into(), Command::Refresh),
                (NamedKey::F7.into(), Command::SpellCheck),
                (NamedKey::F8.into(), Command::Debug),
                (NamedKey::F10.into(), Command::Menu),
                (NamedKey::F11.into(), Command::Fullscreen),
            ];
            map.extend(shortcuts.iter().cloned());
        }

        // Shift
        #[cfg(not(target_os = "macos"))]
        {
            let modifiers = ModifiersState::SHIFT;
            let map = self.map.entry(modifiers).or_insert_with(Default::default);
            map.insert(NamedKey::F3.into(), Command::FindPrevious);
        }

        // Alt (Option on MacOS)
        let modifiers = ModifiersState::ALT;
        let map = self.map.entry(modifiers).or_insert_with(Default::default);
        #[cfg(not(target_os = "macos"))]
        {
            let shortcuts = [
                (NamedKey::F4.into(), Command::Close),
                (NamedKey::ArrowLeft.into(), Command::NavPrevious),
                (NamedKey::ArrowRight.into(), Command::NavNext),
                (NamedKey::ArrowUp.into(), Command::NavParent),
                (NamedKey::ArrowDown.into(), Command::NavDown),
            ];
            map.extend(shortcuts.iter().cloned());
        }
        #[cfg(target_os = "macos")]
        {
            // Missing functionality: move to start/end of paragraph on (Shift)+Alt+Up/Down
            let shortcuts = [
                (NamedKey::ArrowLeft.into(), Command::WordLeft),
                (NamedKey::ArrowRight.into(), Command::WordRight),
            ];

            map.insert(NamedKey::Delete.into(), Command::DelWordBack);
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
            (Key::Character("w".into()), Command::Close),
            (Key::Character("x".into()), Command::Cut),
            (Key::Character("z".into()), Command::Undo),
            (NamedKey::Tab.into(), Command::TabNext),
        ];
        map.extend(shortcuts.iter().cloned());
        #[cfg(target_os = "macos")]
        {
            let shortcuts = [
                (Key::Character("g".into()), Command::FindNext),
                (NamedKey::ArrowUp.into(), Command::DocHome),
                (NamedKey::ArrowDown.into(), Command::DocEnd),
                (NamedKey::ArrowLeft.into(), Command::Home),
                (NamedKey::ArrowRight.into(), Command::End),
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
                (NamedKey::ArrowUp.into(), Command::ViewUp),
                (NamedKey::ArrowDown.into(), Command::ViewDown),
                (NamedKey::ArrowLeft.into(), Command::WordLeft),
                (NamedKey::ArrowRight.into(), Command::WordRight),
                (NamedKey::Backspace.into(), Command::DelWordBack),
                (NamedKey::Delete.into(), Command::DelWord),
                (NamedKey::Home.into(), Command::DocHome),
                (NamedKey::End.into(), Command::DocEnd),
                (NamedKey::PageUp.into(), Command::TabPrevious),
                (NamedKey::PageDown.into(), Command::TabNext),
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
            (NamedKey::Tab.into(), Command::TabPrevious),
        ];
        map.extend(shortcuts.iter().cloned());
        #[cfg(target_os = "macos")]
        {
            let shortcuts = [
                (Key::Character("g".into()), Command::FindPrevious),
                (Key::Character(":".into()), Command::SpellCheck),
                (NamedKey::ArrowUp.into(), Command::DocHome),
                (NamedKey::ArrowDown.into(), Command::DocEnd),
                (NamedKey::ArrowLeft.into(), Command::Home),
                (NamedKey::ArrowRight.into(), Command::End),
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
mod common {
    use super::{Command, Key, ModifiersState, NamedKey};
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;
    use serde::{Deserialize, Serialize};
    use std::fmt;
    use winit::keyboard::{NativeKey, SmolStr};

    /// A subset of [`Key`] which serialises to a simple value usable as a map key
    #[derive(Deserialize)]
    #[serde(untagged)]
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) enum SimpleKey {
        Named(NamedKey),
        Char(char),
    }

    impl From<SimpleKey> for Key<SmolStr> {
        fn from(sk: SimpleKey) -> Self {
            match sk {
                SimpleKey::Named(key) => Key::Named(key),
                SimpleKey::Char(c) => {
                    let mut buf = [0; 4];
                    let s = c.encode_utf8(&mut buf);
                    Key::Character(SmolStr::new(s))
                }
            }
        }
    }

    // NOTE: the only reason we don't use derive is that TOML does not support char as a map key,
    // thus we must convrt with char::encode_utf8. See toml-lang/toml#1001
    impl Serialize for SimpleKey {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            match self {
                SimpleKey::Named(key) => key.serialize(s),
                SimpleKey::Char(c) => {
                    let mut buf = [0; 4];
                    let cs = c.encode_utf8(&mut buf);
                    s.serialize_str(cs)
                }
            }
        }
    }

    /// A subset of [`Key`], excluding anything which is a [`SimpleKey`]
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) enum ComplexKey<Str> {
        Character(Str),
        Dead(char),
        #[serde(untagged)]
        Unidentified(NativeKey),
    }

    impl From<ComplexKey<SmolStr>> for Key<SmolStr> {
        fn from(ck: ComplexKey<SmolStr>) -> Self {
            match ck {
                ComplexKey::Character(c) => Key::Character(c),
                ComplexKey::Dead(c) => Key::Dead(Some(c)),
                ComplexKey::Unidentified(code) => Key::Unidentified(code),
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) struct ModifiersStateDeser(pub ModifiersState);

    impl Serialize for ModifiersStateDeser {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            const SHIFT: ModifiersState = ModifiersState::SHIFT;
            const CONTROL: ModifiersState = ModifiersState::CONTROL;
            const ALT: ModifiersState = ModifiersState::ALT;
            const SUPER: ModifiersState = ModifiersState::SUPER;

            let s = match self.0 {
                state if state == ModifiersState::empty() => "none",
                SUPER => "super",
                ALT => "alt",
                state if state == ALT | SUPER => "alt-super",
                state if state == CONTROL => "ctrl",
                state if state == CONTROL | SUPER => "ctrl-super",
                state if state == CONTROL | ALT => "ctrl-alt",
                state if state == CONTROL | ALT | SUPER => "ctrl-alt-super",
                SHIFT => "shift",
                state if state == SHIFT | SUPER => "shift-super",
                state if state == SHIFT | ALT => "alt-shift",
                state if state == SHIFT | ALT | SUPER => "alt-shift-super",
                state if state == SHIFT | CONTROL => "ctrl-shift",
                state if state == SHIFT | CONTROL | SUPER => "ctrl-shift-super",
                state if state == SHIFT | CONTROL | ALT => "ctrl-alt-shift",
                _ => "ctrl-alt-shift-super",
            };

            serializer.serialize_str(s)
        }
    }

    impl<'de> Visitor<'de> for ModifiersStateDeser {
        type Value = ModifiersStateDeser;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("none or (sub-set of) ctrl-alt-shift-super")
        }

        fn visit_str<E: de::Error>(self, u: &str) -> Result<Self::Value, E> {
            let mut v = u;
            let mut state = ModifiersState::empty();

            let adv_dash_if_not_empty = |v: &mut &str| {
                if !v.is_empty() && v.starts_with('-') {
                    *v = &v[1..];
                }
            };

            if v.starts_with("ctrl") {
                state |= ModifiersState::CONTROL;
                v = &v[v.len().min(4)..];
                adv_dash_if_not_empty(&mut v);
            }
            if v.starts_with("alt") {
                state |= ModifiersState::ALT;
                v = &v[v.len().min(3)..];
                adv_dash_if_not_empty(&mut v);
            }
            if v.starts_with("shift") {
                state |= ModifiersState::SHIFT;
                v = &v[v.len().min(5)..];
                adv_dash_if_not_empty(&mut v);
            }
            if v.starts_with("super") {
                state |= ModifiersState::SUPER;
                v = &v[v.len().min(5)..];
            }

            if v.is_empty() || u == "none" {
                Ok(ModifiersStateDeser(state))
            } else {
                Err(E::invalid_value(
                    de::Unexpected::Str(u),
                    &"none or (sub-set of) ctrl-alt-shift-super",
                ))
            }
        }
    }

    impl<'de> Deserialize<'de> for ModifiersStateDeser {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_str(ModifiersStateDeser(Default::default()))
        }
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) struct MiscRule<Str = SmolStr> {
        #[serde(rename = "modifiers")]
        pub(super) mods: ModifiersStateDeser,
        #[serde(flatten)]
        pub(super) key: ComplexKey<Str>,
        #[serde(rename = "command")]
        pub(super) cmd: Command,
    }
}

#[cfg(feature = "serde")]
mod ser {
    use super::common::{ComplexKey, MiscRule, ModifiersStateDeser, SimpleKey};
    use super::{Key, Shortcuts};
    use serde::ser::{Serialize, SerializeMap, Serializer};

    fn unpack_key<'a>(key: Key<&'a str>) -> Result<SimpleKey, ComplexKey<&'a str>> {
        match key {
            Key::Named(named) => Ok(SimpleKey::Named(named)),
            Key::Character(c) => {
                let mut iter = c.chars();
                if let Some(c) = iter.next()
                    && iter.next().is_none()
                {
                    return Ok(SimpleKey::Char(c));
                }
                Err(ComplexKey::Character(c))
            }
            Key::Unidentified(code) => Err(ComplexKey::Unidentified(code)),
            Key::Dead(None) => panic!("invalid shortcut"),
            Key::Dead(Some(c)) => Err(ComplexKey::Dead(c)),
        }
    }

    impl Serialize for Shortcuts {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            // Use BTreeMap for stable order of output
            use std::collections::BTreeMap;

            let mut serializer = s.serialize_map(Some(self.map.len() + 1))?;
            let mut misc = Vec::new();

            for (state, key_cmds) in self.map.iter() {
                let mods = ModifiersStateDeser(*state);
                let mut map = BTreeMap::new();

                for (key, cmd) in key_cmds.iter() {
                    match unpack_key(key.as_ref()) {
                        Ok(sk) => {
                            map.insert(sk, *cmd);
                        }
                        Err(key) => {
                            let cmd = *cmd;
                            misc.push(MiscRule { mods, key, cmd });
                        }
                    }
                }

                // Keys are now sorted and filtered
                if !map.is_empty() {
                    serializer.serialize_key(&mods)?;
                    serializer.serialize_value(&map)?;
                }
            }

            if !misc.is_empty() {
                serializer.serialize_key("other")?;
                misc.sort();
                serializer.serialize_value(&misc)?;
            }
            serializer.end()
        }
    }
}

#[cfg(feature = "serde")]
mod deser {
    use super::common::{MiscRule, ModifiersStateDeser, SimpleKey};
    use super::{Command, Key, ModifiersState, Shortcuts};
    use linear_map::LinearMap;
    use serde::de::{self, Deserialize, DeserializeSeed, Deserializer, MapAccess, Visitor};
    use std::collections::HashMap;
    use std::fmt;

    enum OptModifiersStateDeser {
        State(ModifiersStateDeser),
        Other,
    }

    impl<'de> Deserialize<'de> for OptModifiersStateDeser {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_str(OptModifiersStateDeser::Other)
        }
    }

    impl<'de> Visitor<'de> for OptModifiersStateDeser {
        type Value = OptModifiersStateDeser;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("none or (sub-set of) ctrl-alt-shift-super or other")
        }

        fn visit_str<E: de::Error>(self, u: &str) -> Result<Self::Value, E> {
            if u == "other" {
                Ok(OptModifiersStateDeser::Other)
            } else {
                ModifiersStateDeser::visit_str(ModifiersStateDeser(Default::default()), u)
                    .map(OptModifiersStateDeser::State)
            }
        }
    }

    struct DeserSimple<'a>(&'a mut HashMap<Key, Command>);

    impl<'a, 'de> DeserializeSeed<'de> for DeserSimple<'a> {
        type Value = ();

        fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_map(self)
        }
    }

    impl<'a, 'de> Visitor<'de> for DeserSimple<'a> {
        type Value = ();

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<A>(self, mut reader: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            while let Some(sk) = reader.next_key::<SimpleKey>()? {
                let key: Key = sk.into();
                let cmd: Command = reader.next_value()?;
                self.0.insert(key, cmd);
            }

            Ok(())
        }
    }

    struct DeserComplex<'a>(&'a mut LinearMap<ModifiersState, HashMap<Key, Command>>);

    impl<'a, 'de> DeserializeSeed<'de> for DeserComplex<'a> {
        type Value = ();

        fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_seq(self)
        }
    }

    impl<'a, 'de> Visitor<'de> for DeserComplex<'a> {
        type Value = ();

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            while let Some(rule) = seq.next_element::<MiscRule>()? {
                let ModifiersStateDeser(state) = rule.mods;
                let sub = self.0.entry(state).or_insert_with(Default::default);
                sub.insert(rule.key.into(), rule.cmd);
            }

            Ok(())
        }
    }

    struct ShortcutsVisitor;

    impl<'de> Visitor<'de> for ShortcutsVisitor {
        type Value = Shortcuts;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<A>(self, mut reader: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut map = LinearMap::<ModifiersState, HashMap<Key, Command>>::new();
            while let Some(opt_state) = reader.next_key::<OptModifiersStateDeser>()? {
                match opt_state {
                    OptModifiersStateDeser::State(ModifiersStateDeser(state)) => {
                        let sub = map.entry(state).or_insert_with(Default::default);
                        reader.next_value_seed(DeserSimple(sub))?;
                    }
                    OptModifiersStateDeser::Other => {
                        reader.next_value_seed(DeserComplex(&mut map))?;
                    }
                }
            }

            Ok(Shortcuts { map })
        }
    }

    impl<'de> Deserialize<'de> for Shortcuts {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_map(ShortcutsVisitor)
        }
    }
}
