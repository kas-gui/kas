// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Configuration context

use crate::event::EventState;
use crate::text::format::FormattableText;
use crate::theme::{SizeCx, Text, ThemeSize};
use crate::{ActionResize, Id, Node};
use std::any::TypeId;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

#[allow(unused)] use crate::Events;

/// Widget configuration and update context
///
/// This type supports access to [`EventState`] via [`Deref`] / [`DerefMut`]
/// and to [`SizeCx`] via [`Self::size_cx`].
#[must_use]
pub struct ConfigCx<'a> {
    pub(super) theme: &'a dyn ThemeSize,
    pub(crate) state: &'a mut EventState,
    pub(crate) resize: Option<ActionResize>,
    pub(crate) redraw: bool,
}

impl<'a> ConfigCx<'a> {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub fn new(sh: &'a dyn ThemeSize, ev: &'a mut EventState) -> Self {
        ConfigCx {
            theme: sh,
            state: ev,
            resize: None,
            redraw: false,
        }
    }

    /// Access a [`SizeCx`]
    #[inline]
    pub fn size_cx<'b>(&'b mut self) -> SizeCx<'b>
    where
        'a: 'b,
    {
        SizeCx::new(self.state, self.theme)
    }

    /// Configure a widget
    ///
    /// All widgets must be configured after construction; see
    /// [widget lifecycle](crate::Widget#widget-lifecycle) and
    /// [configuration](Events#configuration).
    /// Widgets must always be sized after configuration.
    ///
    /// Assigns `id` to the widget. This must be valid and is usually
    /// constructed with [`Events::make_child_id`].
    #[inline]
    pub fn configure(&mut self, mut widget: Node<'_>, id: Id) {
        // This recurses; avoid passing existing state in
        // (Except redraw: this doesn't matter.)
        let start_resize = std::mem::take(&mut self.resize);
        widget._configure(self, id);
        self.resize = self.resize.or(start_resize);
    }

    /// Update a widget
    ///
    /// All widgets must be updated after input data changes; see
    /// [update](Events#update).
    #[inline]
    pub fn update(&mut self, mut widget: Node<'_>) {
        // This recurses; avoid passing existing state in
        // (Except redraw: this doesn't matter.)
        let start_resize = std::mem::take(&mut self.resize);
        widget._update(self);
        self.resize = self.resize.or(start_resize);
    }

    /// Configure a text object
    ///
    /// Font selection depends on the [`TextClass`], [theme configuration] and
    /// the loaded [fonts][crate::text::fonts]. Font size depends on the
    /// [`TextClass`], [theme configuration] and scale factor.
    ///
    /// [`TextClass`]: crate::theme::TextClass
    /// [theme configuration]: crate::config::ThemeConfig
    #[inline]
    pub fn text_configure<T: FormattableText>(&self, text: &mut Text<T>) {
        let class = text.class();
        self.theme.text_configure(text, class);
    }

    /// Configure a text object with custom font size
    ///
    /// Font selection depends on the [`TextClass`], [theme configuration] and
    /// the loaded [fonts][crate::text::fonts].
    ///
    /// Font size must be specified in `dpem`: physical pixels per font Em.
    /// The default font size is available through [`SizeCx::dpem`].
    ///
    /// [`TextClass`]: crate::theme::TextClass
    /// [theme configuration]: crate::config::ThemeConfig
    #[inline]
    pub fn text_configure_with_dpem<T: FormattableText>(&self, text: &mut Text<T>, dpem: f32) {
        let class = text.class();
        self.theme.text_configure_with_dpem(text, class, dpem);
    }

    /// Set a target for messages of a specific type when sent to `Id::default()`
    ///
    /// Messages of this type sent to `Id::default()` from any window will be
    /// sent to `id`.
    pub fn set_send_target_for<M: Debug + 'static>(&mut self, id: Id) {
        let type_id = TypeId::of::<M>();
        self.pending_send_targets.push((type_id, id));
    }

    /// Notify that a widget must be redrawn
    ///
    /// "The current widget" is inferred from the widget tree traversal through
    /// which the `EventCx` is made accessible. The resize is handled locally
    /// during the traversal unwind if possible.
    #[inline]
    pub fn redraw(&mut self) {
        self.redraw = true;
    }

    /// Require that the current widget (and its descendants) be resized
    ///
    /// "The current widget" is inferred from the widget tree traversal through
    /// which the `EventCx` is made accessible. The resize is handled locally
    /// during the traversal unwind if possible.
    #[inline]
    pub fn resize(&mut self) {
        self.resize = Some(ActionResize);
    }
}

impl<'a> Deref for ConfigCx<'a> {
    type Target = EventState;
    fn deref(&self) -> &EventState {
        self.state
    }
}
impl<'a> DerefMut for ConfigCx<'a> {
    fn deref_mut(&mut self) -> &mut EventState {
        self.state
    }
}
