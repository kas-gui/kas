// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Core data types

use super::Id;
use crate::geom::Rect;
#[allow(unused)] use crate::{Events, Layout, Widget};
use std::ops::{Range, RangeInclusive};

/// An opaque type representing a set of `usize` indices
///
/// The only representation currently supported is a range.
//
// NOTE: this API is extensible to other representations like an enum over
// Range or Box<[usize]> (or Vec<usize>).
#[derive(Clone, Debug)]
pub struct ChildIndices(usize, usize);

impl ChildIndices {
    /// Construct: no indices
    #[inline]
    pub fn none() -> Self {
        ChildIndices(0, 0)
    }

    /// Construct: one index
    #[inline]
    pub fn one(index: usize) -> Self {
        ChildIndices(index, index + 1)
    }

    /// Construct: a range
    #[inline]
    pub fn range(range: impl Into<Self>) -> Self {
        range.into()
    }

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

impl From<RangeInclusive<usize>> for ChildIndices {
    #[inline]
    fn from(range: RangeInclusive<usize>) -> Self {
        ChildIndices(*range.start(), *range.end() + 1)
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

/// Type of the widget's core
///
/// This is a special placeholder macro for usage only with the [`widget`](crate::widget) macro.
/// It expands to a type, dependant on the current widget.
///
/// This type always implements the [`WidgetCore`] trait.
///
/// This type *may* implement the [`WidgetCoreRect`] trait.
///
/// # Example
///
/// ```rust
/// # extern crate kas_core as kas;
/// use kas::{impl_self, Events};
///
/// #[impl_self]
/// mod MyHelloWidget {
///     /// A simple greeting
///     #[widget]
///     #[layout("Hello!")]
///     struct MyHelloWidget(widget_core!());
///
///     impl Events for Self {
///         type Data = ();
///     }
/// }
/// ```
#[macro_export]
macro_rules! widget_core {
    () => {
        compile_error!(
            "This macro may only be used in a struct affected by the `#[widget]` attribute"
        );
    };
}

/// Operations supported by a widget core
pub trait WidgetCore: Default {
    /// Get a reference to the widget's identifier
    ///
    /// The widget identifier is assigned when the widget is configured (see
    /// [`Events#configuration`]). In case the
    /// [`Id`] is accessed before this, it will be [invalid](Id#invalid-state).
    /// The identifier *may* change when widgets which are descendants of some
    /// dynamic layout are reconfigured.
    fn id_ref(&self) -> &Id;

    /// Get the widget's identifier
    ///
    /// This method returns a [`Clone`] of [`Self::id_ref`]. Since cloning an
    /// `Id` is [very cheap](Id#representation), this can mostly be ignored.
    ///
    /// The widget identifier is assigned when the widget is configured (see
    /// [`Events#configuration`]). In case the
    /// [`Id`] is accessed before this, it will be [invalid](Id#invalid-state).
    /// The identifier *may* change when widgets which are descendants of some
    /// dynamic layout are reconfigured.
    #[inline]
    fn id(&self) -> Id {
        self.id_ref().clone()
    }

    /// Get the widget configuration status
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn status(&self) -> WidgetStatus;

    /// Require configuration status of at least `status`
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    #[inline]
    fn require_status(&self, status: WidgetStatus) {
        self.status().require(self.id_ref(), status);
    }

    /// Require that [`Layout::size_rules`] has been called for both axes
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    #[inline]
    fn require_status_size_rules(&self) {
        self.require_status(WidgetStatus::SizeRulesY);
    }

    /// Require that [`Layout::set_rect`] has been called
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    #[inline]
    fn require_status_set_rect(&self) {
        self.require_status(WidgetStatus::SetRect);
    }
}

/// Extension for a widget core with a [`Rect`]
///
/// This is only implemented on the core when there is not an explicit
/// definition of [`Layout::rect`].
pub trait WidgetCoreRect: WidgetCore {
    /// Get the stored [`Rect`]
    ///
    /// This should be equivalent to [`Layout::rect`].
    fn rect(&self) -> Rect;

    /// Set the stored [`Rect`]
    fn set_rect(&mut self, rect: Rect);
}

/// Common widget data
///
/// This type may be used for a [`Widget`]'s `core: widget_core!()` field.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
#[derive(Default, Debug)]
pub struct DefaultCoreType {
    pub _id: Id,
    pub status: WidgetStatus,
}

impl WidgetCore for DefaultCoreType {
    #[inline]
    fn id_ref(&self) -> &Id {
        &self._id
    }

    #[inline]
    fn status(&self) -> WidgetStatus {
        self.status
    }
}

/// Common widget data
///
/// This type may be used for a [`Widget`]'s `core: widget_core!()` field.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
#[derive(Default, Debug)]
pub struct DefaultCoreRectType {
    pub _rect: Rect,
    pub _id: Id,
    pub status: WidgetStatus,
}

impl WidgetCore for DefaultCoreRectType {
    #[inline]
    fn id_ref(&self) -> &Id {
        &self._id
    }

    #[inline]
    fn status(&self) -> WidgetStatus {
        self.status
    }
}

impl WidgetCoreRect for DefaultCoreRectType {
    #[inline]
    fn rect(&self) -> Rect {
        self._rect
    }

    #[inline]
    fn set_rect(&mut self, rect: Rect) {
        self._rect = rect;
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum WidgetStatus {
    #[default]
    New,
    Configured,
    SizeRulesX,
    SizeRulesY,
    SetRect,
}

impl WidgetStatus {
    fn require(self, id: &Id, expected: Self) {
        if self < expected {
            panic!("WidgetStatus of {id}: require {expected:?}, found {self:?}");
        }
    }

    /// Configure
    ///
    /// Requires no prior state. Does not imply further actions.
    #[inline]
    pub fn set_configured(&mut self) {
        // re-configure does not require repeating other actions
        *self = (*self).max(WidgetStatus::Configured);
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
    #[inline]
    pub fn set_sized(&mut self) {
        *self = WidgetStatus::SetRect;
    }

    /// Get whether the widget is configured
    #[inline]
    pub fn is_configured(self) -> bool {
        self >= WidgetStatus::Configured
    }

    /// Get whether the widget is sized
    #[inline]
    pub fn is_sized(self) -> bool {
        self >= WidgetStatus::SetRect
    }
}
