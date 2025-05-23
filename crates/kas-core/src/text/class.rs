// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text classes

use std::borrow::Cow;
use std::hash::Hasher;
use std::cmp::{Ordering, PartialEq, PartialOrd};

/// Class key
///
/// This is a name plus a pre-computed hash value. It is used for font mapping.
//
// NOTE: the requirement to serialize this makes things much more complex:
// either we need an owning variant or we need a registry of all possible values
// before deserialization (which happens before the UI is constructed).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "SerializeName"))]
#[derive(Clone, Debug, Eq)]
pub struct Key(Cow<'static, str>, u64);

impl Key {
    /// Construct a key
    pub const fn new(name: &'static str) -> Self {
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(name);
        Key(Cow::Borrowed(name), hash)
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        // TODO: this requires that we check for collisions somewhere
        self.1 == other.1
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Key) -> Option<Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Key) -> Ordering {
        self.1.cmp(&other.1)
    }
}

impl std::hash::Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.1);
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
struct SerializeName(String);

#[cfg(feature = "serde")]
impl From<Key> for SerializeName {
    fn from(key: Key) -> Self {
        SerializeName(key.0.to_string())
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
struct SerializedName(String);

#[cfg(feature = "serde")]
impl From<SerializedName> for Key {
    fn from(name: SerializedName) -> Self {
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(&name.0);
        Key(Cow::Owned(name.0), hash)
    }
}

/// A [`Hasher`] optimized for [`Key`]
///
/// Warning: this hasher should only be used for keys of type [`Key`].
/// In most other cases it will panic or give poor results.
#[derive(Default)]
pub struct KeyHasher(u64);

impl Hasher for KeyHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unimplemented!()
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        debug_assert!(self.0 == 0);
        self.0 = i;
    }
}

/// A hash builder for [`KeyHasher`]
pub type BuildKeyHasher = std::hash::BuildHasherDefault<KeyHasher>;

// /// Text wrap mode
// enum WrapMode {
//     /// Do not break long lines
//     None,
//
// }

/// A text class
pub trait TextClass {
    /// Each text class must have a unique key, used for lookups
    const KEY: Key;

    /// Whether to perform line-breaking
    ///
    /// If `true`, long lines are broken at appropriate positions (see Unicode
    /// UAX 14) to respect some maximum line length.
    ///
    /// If `false`, only explicit line breaks result in new lines.
    fn wrap(&self) -> bool;

    // /// Whether to wrap even where plenty of space is available
    // ///
    // /// If `self.wrap() && self.restrict_width()`, then lines will be wrapped
    // /// at some sensible (theme-defined) maximum paragraph width even when
    // /// plenty of space is availble.
    // fn restrict_width(&self) -> bool;

    /// Whether to enforce a minimum size
    ///
    /// Default value: `false`.
    fn editable(&self) -> bool {
        false
    }

    /// Access key mode
    ///
    /// If `true`, then text decorations (underline, strikethrough) are only
    /// drawn when access key mode is active (usually, this means <kbd>Alt</kbd>
    /// is held).
    ///
    /// Default value: `false`.
    fn is_access_key(&self) -> bool {
        false
    }
}

/// Text class: label
///
/// This will wrap text if and only if the field is true.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Label(pub bool);

impl TextClass for Label {
    const KEY: Key = Key::new("kas::Label");

    fn wrap(&self) -> bool {
        self.0
    }
}

/// Text class: edit field
///
/// This will wrap text if and only if the field is true.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct EditField(pub bool);

impl TextClass for EditField {
    const KEY: Key = Key::new("kas::EditField");

    fn wrap(&self) -> bool {
        self.0
    }
}
