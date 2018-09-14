//! Toolkit interface

use {Coord, Rect};
use widget::window::Window;

/// The type of per-widget toolkit data.
/// 
/// May be used however the toolkit deems fit, except that widgets are allowed
/// to default-construct this (i.e. set to zero).
/// 
/// Toolkits may with to transmute data to/from their own type(s). In this case
/// they should ensure (a) that `size_of::<TkData>()` is sufficient, (b) that
/// `align_of::<TkData>()` is sufficient, (c) gracefully handle the case
/// `TkData` is larger than their type.
#[derive(Clone, Debug, Default)]
pub struct TkData(pub u64);

/// A toolkit handles window management and rendering for a GUI.
/// 
/// Any initialisation should be taken care of in the constructor, and
/// de-initialisation in a `Drop` implementation.
pub trait Toolkit {
    /// Assume ownership of and display a window.
    fn add<W: Clone+Window+'static>(&mut self, window: &W) where Self: Sized;
    
    /// Run the main loop.
    fn main(&mut self);
    
    /// Get a `TkWidget`
    fn tk_widget(&self) -> &TkWidget;
}

/// Common widget properties. Implemented by the toolkit.
pub trait TkWidget {
    /// Get the widget's minimum and preferred sizes.
    fn size_hints(&self, tkd: TkData) -> (Coord, Coord);
    
    /// Set the widget's position and size.
    /// 
    /// Does not need to update child widgets.
    fn set_rect(&self, tkd: TkData, rect: &Rect);
}
