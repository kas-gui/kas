//! Mygui lib

#[doc(hidden)]
#[cfg(feature = "cassowary")]
pub extern crate cassowary as cw;    // used by macros

pub mod event;
pub mod widget;
pub mod toolkit;

mod util;

pub use crate::util::{Coord, Rect};
