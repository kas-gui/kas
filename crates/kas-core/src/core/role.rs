// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget roles

#[allow(unused)] use crate::Tile;
#[allow(unused)] use crate::event::EventState;

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
    /// A window
    Window,
}
