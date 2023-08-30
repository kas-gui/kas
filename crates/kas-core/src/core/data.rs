// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget data types

#[allow(unused)] use super::Widget;
use super::WidgetId;
use crate::geom::Rect;

#[cfg(feature = "winit")] pub use winit::window::Icon;

/// An icon used for the window titlebar, taskbar, etc.
#[cfg(not(feature = "winit"))]
#[derive(Clone)]
pub struct Icon;
#[cfg(not(feature = "winit"))]
impl Icon {
    /// Creates an `Icon` from 32bpp RGBA data.
    ///
    /// The length of `rgba` must be divisible by 4, and `width * height` must equal
    /// `rgba.len() / 4`. Otherwise, this will return a `BadIcon` error.
    pub fn from_rgba(
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<Self, impl std::error::Error> {
        let _ = (rgba, width, height);
        Result::<Self, std::convert::Infallible>::Ok(Icon)
    }
}

/// Common widget data
///
/// This type may be used for a [`Widget`]'s `core: widget_core!()` field.
#[derive(Default, Debug)]
pub struct CoreData {
    pub rect: Rect,
    pub id: WidgetId,
}

/// Note: the clone has default-initialised identifier.
/// Configuration and layout solving is required as for any other widget.
impl Clone for CoreData {
    fn clone(&self) -> Self {
        CoreData {
            rect: self.rect,
            id: WidgetId::default(),
        }
    }
}
