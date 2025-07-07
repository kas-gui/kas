// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget data types

use super::Id;
#[allow(unused)] use super::Widget;
use crate::geom::Rect;
use std::ops::Range;

pub use winit::window::Icon;

/// An opaque type indexible over `usize`
///
/// Currently, the only supported representation is a range. Construct using
/// [`From`] impls, e.g. `(0..self.widgets.len()).into()`.
//
// NOTE: this API is extensible to other representations like an enum over
// Range or Box<[usize]> (or Vec<usize>).
#[derive(Clone, Debug)]
pub struct ChildIndices(usize, usize);

impl ChildIndices {
    // pub fn iter(&self) -> ChildIndicesRefIter<'_> { .. }

    /// Convert to a Range
    #[inline]
    pub(crate) fn as_range(&self) -> Range<usize> {
        self.0..self.1
    }
}

impl IntoIterator for ChildIndices {
    type Item = usize;
    type IntoIter = ChildIndicesIter;

    #[inline]
    fn into_iter(self) -> ChildIndicesIter {
        ChildIndicesIter(self.0..self.1)
    }
}

impl From<Range<usize>> for ChildIndices {
    #[inline]
    fn from(range: Range<usize>) -> Self {
        ChildIndices(range.start, range.end)
    }
}

/// Owning iterator over [`ChildIndices`]
#[derive(Clone, Debug)]
pub struct ChildIndicesIter(Range<usize>);

impl Iterator for ChildIndicesIter {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl ExactSizeIterator for ChildIndicesIter {}
impl DoubleEndedIterator for ChildIndicesIter {
    #[inline]
    fn next_back(&mut self) -> Option<usize> {
        self.0.next_back()
    }
}

/// Common widget data
///
/// This type may be used for a [`Widget`]'s `core: widget_core!()` field.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
#[derive(Default, Debug)]
pub struct DefaultCoreType {
    pub _rect: Rect,
    pub _id: Id,
    #[cfg(debug_assertions)]
    pub status: WidgetStatus,
}

impl Clone for DefaultCoreType {
    fn clone(&self) -> Self {
        DefaultCoreType {
            _rect: self._rect,
            _id: Default::default(),
            #[cfg(debug_assertions)]
            status: self.status,
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
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
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
    fn require(&self, id: &Id, expected: Self) {
        if *self < expected {
            panic!("WidgetStatus of {id}: require {expected:?}, found {self:?}");
        }
    }

    /// Configure
    ///
    /// Requires nothing. Re-configuration does not require repeating other actions.
    pub fn configure(&mut self, _id: &Id) {
        // re-configure does not require repeating other actions
        *self = (*self).max(WidgetStatus::Configured);
    }

    /// Update
    ///
    /// Requires configure. Does not affect status (note that widgets are always
    /// updated immediately after configure, hence `WidgetStatus::Configured`
    /// implies that `update` has been called or is just about to be called).
    pub fn update(&self, id: &Id) {
        self.require(id, WidgetStatus::Configured);

        // Update-after-configure is already guaranteed (see impls module).
        // NOTE: Update-after-data-change should be required but is hard to
        // detect; we could store a data hash but draw does not receive data.
        // As such we don't bother recording this operation.
    }

    /// Size rules
    ///
    /// Requires a prior call to `configure`. When `axis.is_vertical()`,
    /// requires a prior call to `size_rules` for the horizontal axis.
    ///
    /// Re-calling `size_rules` does not require additional actions.
    pub fn size_rules(&mut self, id: &Id, axis: crate::layout::AxisInfo) {
        if axis.is_horizontal() {
            self.require(id, WidgetStatus::Configured);
            *self = (*self).max(WidgetStatus::SizeRulesX);
        } else {
            self.require(id, WidgetStatus::SizeRulesX);
            *self = (*self).max(WidgetStatus::SizeRulesY);
        }
    }

    /// Set rect
    ///
    /// Requires calling `size_rules` for each axis. Re-calling `set_rect` does
    /// not require additional actions.
    pub fn set_rect(&mut self, id: &Id) {
        self.require(id, WidgetStatus::SizeRulesY);
        *self = WidgetStatus::SetRect;
    }

    /// Require that `set_rect` has been called
    pub fn require_rect(&self, id: &Id) {
        self.require(id, WidgetStatus::SetRect);
    }
}
