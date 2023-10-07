// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget data types

#[allow(unused)] use super::Widget;
use super::{OwnedId, Id};
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
    pub id: OwnedId,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[cfg(debug_assertions)]
    pub status: WidgetStatus,
}

/// Note: the clone has default-initialised identifier.
/// Configuration and layout solving is required as for any other widget.
impl Clone for CoreData {
    fn clone(&self) -> Self {
        CoreData {
            rect: self.rect,
            ..CoreData::default()
        }
    }
}

/// Widget state tracker
///
/// This struct is used to track status of widget operations and panic in case
/// of inappropriate call order (such cases are memory safe but may cause
/// incorrect widget behaviour).
///
/// It is not used in release builds.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum WidgetStatus {
    #[default]
    New,
    Configured,
    SizeRulesX,
    SizeRulesY,
    SetRect,
}

#[cfg(debug_assertions)]
impl WidgetStatus {
    fn require(&self, id: Id, expected: Self) {
        if *self < expected {
            panic!("WidgetStatus of {id}: require {expected:?}, found {self:?}");
        }
    }

    /// Configure
    pub fn configure(&mut self, _id: Id) {
        // re-configure does not require repeating other actions
        *self = (*self).max(WidgetStatus::Configured);
    }

    /// Update
    pub fn update(&self, id: Id) {
        self.require(id, WidgetStatus::Configured);

        // Update-after-configure is already guaranteed (see impls module).
        // NOTE: Update-after-data-change should be required but is hard to
        // detect; we could store a data hash but draw does not receive data.
        // As such we don't bother recording this operation.
    }

    /// Size rules
    pub fn size_rules(&mut self, id: Id, axis: crate::layout::AxisInfo) {
        // NOTE: Possibly this is too strict and we should not require
        // re-running size_rules(vert) or set_rect afterwards?
        if axis.is_horizontal() {
            self.require(id, WidgetStatus::Configured);
            *self = WidgetStatus::SizeRulesX;
        } else {
            self.require(id, WidgetStatus::SizeRulesX);
            *self = WidgetStatus::SizeRulesY;
        }
    }

    /// Set rect
    pub fn set_rect(&mut self, id: Id) {
        self.require(id, WidgetStatus::SizeRulesY);
        *self = WidgetStatus::SetRect;
    }

    pub fn require_rect(&self, id: Id) {
        self.require(id, WidgetStatus::SetRect);
    }
}
