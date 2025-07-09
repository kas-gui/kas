// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget roles

#[allow(unused)] use crate::Tile;
use crate::dir::Direction;
#[allow(unused)] use crate::event::EventState;
use crate::geom::Offset;

/// Describes a widget's purpose and capabilities
///
/// This `enum` does not describe children; use [`Tile::child_indices`] for
/// that. This `enum` does not describe associated properties such as a label
/// or labelled-by relationship.
///
/// ### Messages
///
/// Some roles of widget are expected to accept specific messages, as outlined
/// below. See also [`EventState::send`] and related functions.
#[non_exhaustive]
pub enum Role<'a> {
    /// Role is unspecified or no listed role is applicable
    Unknown,
    /// A text label with the given contents, usually (but not necessarily) short and fixed
    Label(&'a str),
    /// A push button
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to trigger the button.
    Button,
    /// A checkable box
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to toggle the state.
    CheckBox(bool),
    /// A radio button
    RadioButton(bool),
    /// A tab handle
    Tab,
    /// A scrollable region
    ScrollRegion {
        /// The current scroll offset (from zero to `max_offset`)
        offset: Offset,
        /// The maximum offset (non-negative)
        max_offset: Offset,
    },
    /// A scroll bar
    ScrollBar {
        /// Orientation (usually either `Down` or `Right`)
        direction: Direction,
        /// The current position (from zero to `max_value`)
        value: i32,
        /// The maximum position (non-negative)
        max_value: i32,
    },
    /// A window
    Window,
}
