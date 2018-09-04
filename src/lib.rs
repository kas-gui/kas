//! Mygui lib

// TODO: for now there are many unused things
#![allow(unused)]

extern crate gdk;
extern crate gtk;

pub mod event;
pub mod widget;
pub mod toolkit;

mod util;

pub use util::Rect;
