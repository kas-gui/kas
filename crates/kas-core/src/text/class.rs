// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text classes

use std::hash::Hasher;
use std::cmp::{Ordering, PartialEq, PartialOrd};

/// Class key
///
/// This is a name plus a pre-computed hash value. It is used for font mapping.
#[derive(Copy, Clone, Debug, Eq)]
pub struct Key(&'static str, u64);

impl Key {
    /// Construct a key
    pub const fn new(name: &'static str) -> Self {
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(name);
        Key(name, hash)
    }
}

impl PartialEq for Key {
    fn eq(&self, rhs: &Self) -> bool {
        // NOTE: if we test for collisions we could skip testing against field 0
        self.1 == rhs.1 && self.0 == rhs.0
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, rhs: &Key) -> Option<Ordering> {
        self.1.partial_cmp(&rhs.1)
    }
}

impl Ord for Key {
    fn cmp(&self, rhs: &Key) -> Ordering {
        self.1.cmp(&rhs.1)
    }
}

impl std::hash::Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.1);
    }
}

/// A [`Hasher`] optimized for [`Key`]
///
/// Warning: this hasher should only be used for keys of type [`Key`].
/// In most other cases it will panic or give poor results.
#[derive(Default)]
pub(crate) struct KeyHasher(u64);

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
pub(crate) type BuildKeyHasher = std::hash::BuildHasherDefault<KeyHasher>;

bitflags! {
    /// Text class properties
    #[must_use]
    #[derive(Copy, Clone, Debug, Default)]
    pub struct Properties: u32 {
        /// Perform line-wrapping
        ///
        /// If `true`, long lines are broken at appropriate positions (see
        /// Unicode UAX 14) to respect some maximum line length.
        ///
        /// If `false`, only explicit line breaks result in new lines.
        const WRAP = 1 << 0;

        /// Is an access key
        ///
        /// If `true`, then text decorations (underline, strikethrough) are only
        /// drawn when access key mode is active (usually, this means
        /// <kbd>Alt</kbd> is held).
        const ACCESS = 1 << 8;

        /// Limit minimum size
        ///
        /// This is used to prevent empty edit-fields from collapsing to nothing.
        const LIMIT_MIN_SIZE = 1 << 9;
    }
}

/// A text class
pub trait TextClass {
    /// Each text class must have a unique key, used for lookups
    const KEY: Key;

    /// Get text properties
    fn properties(&self) -> Properties;

    // /// Whether to wrap even where plenty of space is available
    // ///
    // /// If `self.wrap() && self.restrict_width()`, then lines will be wrapped
    // /// at some sensible (theme-defined) maximum paragraph width even when
    // /// plenty of space is availble.
    // fn restrict_width(&self) -> bool;
}

/// Text class: label
///
/// This will wrap long lines if and only if its field is true.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Label(pub bool);

impl TextClass for Label {
    const KEY = Key::new("Label");

    fn properties(&self) -> Properties {
        if self.0 {
            Properties::WRAP
        } else {
            Properties::empty()
        }
    }
}

/// Text class: access label
///
/// This is identical to [`Label`] except that effects are only drawn if
/// access key mode is activated (usually the `Alt` key).
///
/// This will wrap long lines if and only if its field is true.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct AccessLabel(pub bool);

/// Text class: scrollable label
///
/// The occupied vertical space may be less than the height of the text object.
/// This will wrap long lines.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct ScrollLabel(pub bool);

/// Text class: menu label
///
/// This is equivalent to [`AccessLabel`] `(false)`, but may use different
/// styling and does not stretch to fill extra space.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct MenuLabel;

/// Text class: button
///
/// This is equivalent to [`AccessLabel`] `(false)`, but may use different
/// styling.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Button;

/// Text class: edit field
///
/// This is a multi-line edit field if and only if its field is true.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Edit(pub bool);
