//! Mygui lib

#[doc(hidden)]
#[cfg(feature = "cassowary")]
pub extern crate cassowary as cw;    // used by macros

// interface modules:
pub mod event;
#[macro_use]
pub mod widget;
pub mod toolkit;
mod util;

// widget implementations:
pub mod control;
pub mod display;
pub mod window;


pub use crate::util::{Coord, Rect};
