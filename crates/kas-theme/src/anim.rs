// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Animation helpers

use std::collections::HashMap;
use std::time::Instant;

/// State of edit cursor
#[derive(Clone, Copy, Debug)]
pub struct TextCursor {
    pub byte: usize,
    pub state: bool,
    pub time: Instant,
}

/// State holding theme animation data
#[derive(Debug, Default)]
pub struct AnimState {
    pub text_cursor: HashMap<u64, TextCursor>,
}
