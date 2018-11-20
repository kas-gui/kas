//! Mygui lib
#![feature(unrestricted_attribute_tokens)]

#[doc(hidden)]
#[cfg(feature = "cassowary")]
pub extern crate cassowary as cw;    // used by macros

extern crate mygui_macros;

// internal modules:
#[macro_use]
mod widget;
mod window;
mod toolkit;

// public implementations:
pub mod control;
pub mod dialog;
pub mod display;
pub mod event;

/// Library macros
/// 
/// Note that some of these are re-exports, but it is expected that users depend on this crate only
/// and not `mygui_macros`. All functionality is available via these re-exports.
pub mod macros {
    pub use mygui_macros::{Widget, make_widget};
}

// export most important members directly for convenience and less redundancy:
pub use crate::widget::*;
pub use crate::window::*;
pub use crate::toolkit::*;
