// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text functionality
//!
//! Most of this module is simply a re-export of the [KAS Text] API, hence the
//! lower level of integration than other parts of the library. The [`util`]
//! module is an extension providing some integration.
//!
//! When using a [`Text`] object in a widget, the text *must* be prepared before
//! display by calling [`ConfigMgr::text_set_size`] from [`Layout::set_rect`].
//! Failure to do so may result in text displaying incorrectly or not at all.
//!
//! [KAS Text]: https://github.com/kas-gui/kas-text/

#[allow(unused)]
use kas::{event::ConfigMgr, Layout};

pub use kas_text::*;

mod selection;
pub use selection::SelectionHelper;

mod string;
pub use string::AccelString;
