// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit interface

use crate::widget::{Size, Rect};

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
    fn size_hints(&self, tkd: TkData) -> (Size, Size);
    
    /// Get the widget's position and size.
    fn get_rect(&self, tkd: TkData) -> Rect;
    
    /// Set the widget's position and size.
    /// 
    /// Does not need to update child widgets.
    fn set_rect(&mut self, tkd: TkData, rect: &Rect);
    
    /// Get the widget's boolean state
    fn get_bool(&self, tkd: TkData) -> bool;
    
    /// Set the widget's boolean state
    /// 
    /// As with the [`HasBool`] trait, this can be used for checkboxes,
    /// radio buttons and toggle switches.
    fn set_bool(&mut self, tkd: TkData, state: bool);
    
    /// Set the widget's text
    /// 
    /// As with the [`HasText`] trait, this applies to both labels and text
    /// content, depending on the widget.
    fn set_text(&mut self, tkd: TkData, text: &str);
}
