// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration context

use super::PendingNavFocus;
use crate::event::{EventState, FocusSource};
use crate::geom::{Rect, Size};
use crate::layout::AlignPair;
use crate::messages::Erased;
use crate::text::format::FormattableText;
use crate::theme::{Feature, SizeCx, Text, ThemeSize};
use crate::{Id, Node};
use cast::Cast;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

#[allow(unused)] use crate::event::{Event, EventCx};
#[allow(unused)] use crate::{Events, Layout};

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
            self.pending_nav_focus = PendingNavFocus::Set {
                target: None,
                source: FocusSource::Synthetic,
            };
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

    /// Push a message (replay)
    ///
    /// Unlike [`EventCx::push`], this is not handled while unwinding
    /// from event sending, but via a fresh traversal of the widget tree.
    ///
    /// TODO: `id` should not be part of the function signature?
    pub fn push<M: Debug + 'static>(&mut self, id: Id, msg: M) {
        self.send(id, msg);
    }

    /// Push a type-erased message (replay)
    ///
    /// Unlike [`EventCx::push_erased`], this is not handled while unwinding
    /// from event sending, but via a fresh traversal of the widget tree.
    ///
    /// The message may be [popped](EventCx::try_pop) or
    /// [observed](EventCx::try_observe) from [`Events::handle_messages`]
    /// by the widget itself, its parent, or any ancestor.
    pub fn push_erased(&mut self, id: Id, msg: Erased) {
        self.send_erased(id, msg);
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

    /// Prepare a text object
    ///
    /// Wrap and align text for display at the given `size`.
    ///
    /// Call [`text_configure`][Self::text_configure] before this method.
    #[inline]
    pub fn text_set_size<T: FormattableText>(
        &self,
        text: &mut Text<T>,
        size: Size,
        align: AlignPair,
    ) {
        text.set_align(align.into());
        text.set_bounds(size.cast());
        text.prepare().expect("not configured");
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
