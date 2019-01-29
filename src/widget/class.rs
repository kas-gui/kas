// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget classifications
//! 
//! Each widget must have one of the classifications defined in the [`Class`]
//! enumeration. In some of these cases, the widget must implement additional
//! functionality (usually on itself).

use crate::traits::*;

/// Widget classifications
pub enum Class<'a> {
    Button(&'a HasText),
    Container,
    // Dialog,
    Label(&'a HasText),
    Entry(&'a Editable),
    Frame,
    Window,
}
