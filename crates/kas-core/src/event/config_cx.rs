// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration context

use crate::event::EventState;
use crate::text::format::FormattableText;
use crate::theme::{SizeCx, Text, ThemeSize};
use crate::{Id, Node};
use std::any::TypeId;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

#[allow(unused)] use crate::event::{Event, EventCx};
#[allow(unused)] use crate::{Action, Events};

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
    pub fn size_cx<'b>(&'b mut self) -> SizeCx<'b>
    where
        'a: 'b,
    {
        SizeCx::new(self.ev, self.sh)
    }

    /// Access [`EventState`]
    #[inline]
    pub fn ev_state(&mut self) -> &mut EventState {
        self.ev
    }

    /// Configure a widget
    ///
    /// All widgets must be configured after construction; see
    /// [widget lifecycle](crate::Widget#widget-lifecycle) and
    /// [configuration](Events#configuration).
    ///
    /// Assigns `id` to the widget. This must be valid and is usually
    /// constructed with [`Events::make_child_id`].
    #[inline]
    pub fn configure(&mut self, mut widget: Node<'_>, id: Id) {
        widget._configure(self, id);
    }

    /// Update a widget
    ///
    /// All widgets must be updated after input data changes; see
    /// [update](Events#update).
    #[inline]
    pub fn update(&mut self, mut widget: Node<'_>) {
        widget._update(self);
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

    /// Set a target for messages of a specific type when sent to `Id::default()`
    ///
    /// Messages of this type sent to `Id::default()` from any window will be
    /// sent to `id`.
    pub fn set_send_target_for<M: Debug + 'static>(&mut self, id: Id) {
        let type_id = TypeId::of::<M>();
        self.pending_send_targets.push((type_id, id));
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
