// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit interface

use std::{cell::RefCell, rc::Rc};

use crate::widget::{Coord, Rect};
use crate::window::Window;

/// The type of per-widget toolkit data.
/// 
/// May be used however the toolkit deems fit, except that widgets are allowed
/// to default-construct this (i.e. set to zero), and valid values should not be
/// zero.
/// 
/// Toolkits may with to transmute data to/from their own type(s). In this case
/// they should ensure (a) that `size_of::<TkData>()` is sufficient, (b) that
/// `align_of::<TkData>()` is sufficient, (c) gracefully handle the case
/// `TkData` is larger than their type.
#[derive(Clone, Debug, Default)]
pub struct TkData(pub u64);

impl TkData {
    /// This property is true for default-constructed values but should be false
    /// after the data has been set by the toolkit.
    /// 
    /// Essentially this test is just that all data is zero.
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// The primary trait to be implemented by KAS toolkits.
/// 
/// A toolkit handles window management and rendering for a GUI.
/// 
/// This is considered a "handle" and implementations are required to support
/// `Clone`, however they do not have to support `Send` or `Sync`.
/// 
/// Any initialisation should be taken care of in the constructor, and
/// de-initialisation in a `Drop` implementation.
pub trait Toolkit: Clone {
    /// Assume ownership of and display a window.
    /// 
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    fn add<W: Window + 'static>(&self, window: W) where Self: Sized {
        self.add_rc(Rc::new(RefCell::new(window)))
    }
    
    /// Specialised version of `add`; typically toolkits only need to implement
    /// this.
    fn add_rc(&self, window: Rc<RefCell<dyn Window>>);
    
    /// Run the main loop.
    fn main(&mut self);
}

/// Common widget properties. Implemented by the toolkit.
/// 
/// Users interact with this trait in a few cases, such as implementing widget
/// event handling. In these cases the user is *always* given an existing
/// reference to a `TkWidget`. Mostly this trait is only used internally.
/// 
/// Note that it is not necessary for toolkits to implement all of these
/// methods, depending on which functionality from the library is used.
pub trait TkWidget {
    /// Get the widget's minimum and preferred sizes.
    fn size_hints(&self, tkd: TkData) -> (Coord, Coord);
    
    /// Get the widget's position and size.
    fn get_rect(&self, tkd: TkData) -> Rect;
    
    /// Set the widget's position and size.
    /// 
    /// Does not need to update child widgets.
    fn set_rect(&self, tkd: TkData, rect: &Rect);
    
    /// Get the widget's boolean state
    fn get_bool(&self, tkd: TkData) -> bool;
    
    /// Set the widget's boolean state
    /// 
    /// As with the [`HasBool`] trait, this can be used for checkboxes,
    /// radio buttons and toggle switches.
    fn set_bool(&self, tkd: TkData, state: bool);
    
    /// Set the widget's text
    /// 
    /// As with the [`HasText`] trait, this applies to both labels and text
    /// content, depending on the widget.
    fn set_text(&self, tkd: TkData, text: &str);
}
