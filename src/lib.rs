//! Mygui lib

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

pub use mygui_macros::mygui_impl;
