// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text functionality
//!
//! Most of this module is simply a re-export of the [KAS Text] API, hence the
//! lower level of integration than other parts of the library.
//!
//! [`Text`] objects *must* be configured and prepared before usage, otherwise
//! they may appear empty. Call [`ConfigCx::text_config`] from
//! [`Events::configure`] and [`ConfigCx::text_set_size`] from
//! [`Layout::set_rect`] to set text position and prepare.
//! If text is adjusted, one may use e.g. [`TextApi::prepare`] to update.
//!
//! [KAS Text]: https://github.com/kas-gui/kas-text/

#[allow(unused)] use kas::{event::ConfigCx, Layout};

pub use kas_text::*;

mod selection;
pub use selection::{SelectionAction, SelectionHelper};

mod string;
pub use string::AccessString;
