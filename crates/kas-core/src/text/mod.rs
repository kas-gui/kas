// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text functionality
//!
//! Most of this module is simply a re-export of the [KAS Text] API, hence the
//! lower level of integration than other parts of the library.
//!
//! See also [`crate::theme::Text`] which provides better integration with KAS
//! theming and widget sizing operations.
//!
//! [KAS Text]: https://github.com/kas-gui/kas-text/

pub use kas_text::{
    fonts, format, Align, Direction, Effect, EffectFlags, MarkerPos, MarkerPosIter, NotReady,
    OwningVecIter, Range, Status, Text, TextDisplay, Vec2, DPU,
};

mod selection;
pub use selection::{SelectionAction, SelectionHelper};

mod string;
pub use string::AccessString;
