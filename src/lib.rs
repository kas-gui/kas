//! Mygui lib
#![feature(unrestricted_attribute_tokens)]

#[doc(hidden)]
#[cfg(feature = "cassowary")]
pub extern crate cassowary as cw;    // used by macros

extern crate mygui_macros;

// interface modules:
pub mod event;
#[macro_use]
pub mod widget;
pub mod toolkit;

// widget implementations:
pub mod control;
pub mod display;
pub mod window;

/// Library macros
/// 
/// Note that some of these are re-exports, but it is expected that users depend on this crate only
/// and not `mygui_macros`. All functionality is available via these re-exports.
pub mod macros {
    pub use mygui_macros::{Widget, make_widget};
}
