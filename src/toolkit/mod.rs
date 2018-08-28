//! Toolkit interface

pub mod gtk;

use widget::window::Window;

/// A toolkit handles window management and rendering for a GUI.
/// 
/// Any initialisation should be taken care of in the constructor, and
/// de-initialisation in a `Drop` implementation.
pub trait Toolkit {
    /// Assume ownership of and display a window.
    fn show<W: Window+'static>(&mut self, window: W);
}
