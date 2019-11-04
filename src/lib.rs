// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS, the toolKit Abstraction Library
//!
//! The KAS library is designed for concise, programatic descriptions of GUIs
//! which are translated to some backend "toolkit" on use.

extern crate kas_macros;
extern crate self as kas; // required for reliable self-reference in kas_macros

use std::fmt;

// internal modules:
mod toolkit;
mod traits;

// public implementations:
pub mod class;
pub mod event;
pub mod geom;
pub mod widget;

// macro re-exports
pub mod macros;

// export most important members directly for convenience and less redundancy:
pub use crate::toolkit::*;
pub use crate::traits::*;

/// Widget identifier
///
/// All widgets within a window are assigned a unique numeric identifier. This
/// type may be tested for equality and order.
///
/// Note: identifiers are first assigned when a window is instantiated by the
/// toolkit.
#[derive(Debug, Default, Clone, Copy, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct WidgetId(u32);

impl WidgetId {
    #[doc(hidden)]
    pub const FIRST: WidgetId = WidgetId(1);

    #[doc(hidden)]
    fn next(self) -> Self {
        WidgetId(self.0 + 1)
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}
