// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration context

use crate::event::EventState;
use crate::geom::Rect;
use crate::layout::AlignPair;
use crate::text::format::FormattableText;
use crate::theme::{Feature, SizeCx, Text, ThemeSize};
use crate::{Id, Node};
use std::ops::{Deref, DerefMut};

#[allow(unused)] use crate::event::{Event, EventCx};
#[allow(unused)] use crate::{Action, Events, Layout};

/// Widget configuration and update context
///
/// This type supports easy access to [`EventState`] (via [`Deref`],
/// [`DerefMut`] and [`Self::ev_state`]) as well as [`SizeCx`]
/// ([`Self::size_cx`]).
#[must_use]
pub struct ConfigCx<'a> {
    sh: &'a dyn ThemeSize,
    pub(crate) ev: &'a mut EventState,
}

impl<'a> ConfigCx<'a> {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub fn new(sh: &'a dyn ThemeSize, ev: &'a mut EventState) -> Self {
        ConfigCx { sh, ev }
    }

    /// Access a [`SizeCx`]
    #[inline]
    pub fn size_cx(&self) -> SizeCx<'a> {
        SizeCx::new(self.sh)
    }

    /// Access [`EventState`]
    #[inline]
    pub fn ev_state(&mut self) -> &mut EventState {
        self.ev
    }

    /// Configure a widget
    ///
    /// All widgets must be configured after construction (see
    /// [widget lifecycle](Layout#widget-lifecycle)).
    /// This method performs complete configuration of the widget by calling
    /// [`Events::configure`], [`Events::update`], [`Events::configure_recurse`].
    ///
    /// Pass the `id` to assign to the widget. This is usually constructed with
    /// [`Events::make_child_id`].
    #[inline]
    pub fn configure(&mut self, mut widget: Node<'_>, id: Id) {
        if id.is_valid() {
            widget._configure(self, id);
        }
    }

    /// Update a widget
    ///
    /// All widgets must be updated after input data changes.
    /// This method recursively updates the widget by calling
    /// [`Events::update`] and [`Events::update_recurse`].
    #[inline]
    pub fn update(&mut self, mut widget: Node<'_>) {
        widget._update(self);
    }

    /// Align a feature's rect
    ///
    /// In case the input `rect` is larger than desired on either axis, it is
    /// reduced in size and offset within the original `rect` as is preferred.
    #[inline]
    pub fn align_feature(&self, feature: Feature, rect: Rect, align: AlignPair) -> Rect {
        self.sh.align_feature(feature, rect, align)
    }

    /// Configure a text object
    ///
    /// This selects a font given the [`TextClass`][crate::theme::TextClass],
    /// [theme configuration][crate::config::ThemeConfig] and
    /// the loaded [fonts][crate::text::fonts].
    #[inline]
    pub fn text_configure<T: FormattableText>(&self, text: &mut Text<T>) {
        let class = text.class();
        self.sh.text_configure(text, class);
    }
}

impl<'a> Deref for ConfigCx<'a> {
    type Target = EventState;
    fn deref(&self) -> &EventState {
        self.ev
    }
}
impl<'a> DerefMut for ConfigCx<'a> {
    fn deref_mut(&mut self) -> &mut EventState {
        self.ev
    }
}
