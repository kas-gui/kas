// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Standard messages
//!
//! These are messages that may be sent via [`EventMgr::push`](crate::event::EventMgr::push).

/// Message: select child
///
/// Example: a list supports selection; a child emits this to cause itself to be selected.
#[derive(Clone, Debug)]
pub struct Select;
