// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration context

use super::Pending;
use crate::event::EventState;
use crate::geom::{Rect, Size};
use crate::layout::AlignPair;
use crate::text::TextApi;
use crate::theme::{Feature, SizeCx, TextClass, ThemeSize};
use crate::{Id, Node};
use std::ops::{Deref, DerefMut};

#[allow(unused)] use crate::{event::Event, Events, Layout};

/// Widget configuration and update context
///
/// This type supports easy access to [`EventState`] (via [`Deref`],
/// [`DerefMut`] and [`Self::ev_state`]) as well as [`SizeCx`]
/// ([`Self::size_cx`]) and [`DrawShared`] ([`Self::draw_shared`]).
#[must_use]
pub struct ConfigCx<'a> {
    sh: &'a dyn ThemeSize,
    pub(crate) ev: &'a mut EventState,
}

impl<'a> ConfigCx<'a> {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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

    /// Disable or enable navigation focus
    ///
    /// When nav focus is disabled, [`EventState::nav_focus`] always returns
    /// `None`. Any existing focus is immediately cleared. Both
    /// [`EventState::set_nav_focus`] and [`EventState::next_nav_focus`] will fail to
    /// do anything. Input such as the <kbd>Tab</kbd> key and mouse click
    /// will not set navigation focus.
    pub fn disable_nav_focus(&mut self, disabled: bool) {
        self.ev.config.nav_focus = !disabled;
        if disabled {
            if let Some(id) = self.ev.nav_focus.take() {
                self.pending
                    .push_back(Pending::Send(id, Event::LostNavFocus));
            }
        }
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
        widget._configure(self, id);
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

    /// Prepare a text object
    ///
    /// This sets the text's font, font size, wrapping and optionally alignment,
    /// then performs the text preparation necessary before display.
    ///
    /// Note: setting alignment here is not necessary when the default alignment
    /// is desired or when [`SizeCx::text_rules`] is used.
    #[inline]
    pub fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: Option<AlignPair>,
    ) {
        self.sh.text_set_size(text, class, size, align)
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
