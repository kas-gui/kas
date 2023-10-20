// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Standard messages
//!
//! These are messages that may be sent via [`EventCx::push`](crate::event::EventCx::push).

use crate::event::PhysicalKey;

/// Message: activate
///
/// Example: a button's label has a keyboard shortcut; this message is sent by the label to
/// trigger the button.
///
/// Payload: the key press which caused this message to be emitted, if any.
#[derive(Copy, Clone, Debug)]
pub struct Activate(pub Option<PhysicalKey>);

/// Message: select child
///
/// Example: a list supports selection; a child emits this to cause itself to be selected.
#[derive(Clone, Debug)]
pub struct Select;
