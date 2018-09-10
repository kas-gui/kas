//! Mygui lib

// TODO: for now there are many unused things
#![allow(unused)]

extern crate glib;
extern crate gdk;
extern crate gtk;
extern crate gtk_sys;

pub mod event;
pub mod widget;
pub mod toolkit;

mod util;

pub use util::{Coord, Rect};
