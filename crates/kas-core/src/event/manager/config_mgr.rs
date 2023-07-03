// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size and EventState manager

use super::Pending;
use crate::draw::DrawShared;
use crate::event::EventState;
use crate::geom::{Rect, Size};
use crate::layout::AlignPair;
use crate::shell::Platform;
use crate::text::TextApi;
use crate::theme::{Feature, SizeMgr, TextClass, ThemeSize};
use crate::{Action, NodeMut, WidgetId};
use std::ops::{Deref, DerefMut};

#[allow(unused)] use crate::{event::Event, Layout};

/// Manager used to configure widgets and layout
///
/// This type supports easy access to [`EventState`] (via [`Deref`],
/// [`DerefMut`] and [`Self::ev_state`]) as well as [`SizeMgr`]
/// ([`Self::size_mgr`]) and [`DrawShared`] ([`Self::draw_shared`]).
#[must_use]
pub struct ConfigMgr<'a> {
    sh: &'a dyn ThemeSize,
    ds: &'a mut dyn DrawShared,
    pub(crate) ev: &'a mut EventState,
}

impl<'a> ConfigMgr<'a> {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(sh: &'a dyn ThemeSize, ds: &'a mut dyn DrawShared, ev: &'a mut EventState) -> Self {
        ConfigMgr { sh, ds, ev }
    }

    /// Get the platform
    pub fn platform(&self) -> Platform {
        self.ds.platform()
    }

    /// Access a [`SizeMgr`]
    ///
    /// Warning: sizes are calculated using the window's current scale factor.
    /// This may change, even without user action, since some platforms
    /// always initialize windows with scale factor 1.
    /// See also notes on [`Events::configure`].
    #[inline]
    pub fn size_mgr(&self) -> SizeMgr<'a> {
        SizeMgr::new(self.sh)
    }

    /// Access [`DrawShared`]
    #[inline]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.ds
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
    /// [`Events::configure`]). This method may be used to configure a new
    /// child widget without requiring the whole window to be reconfigured.
    ///
    /// Pass the `id` to assign to the widget: this should be constructed from
    /// the parent's id via [`WidgetId::make_child`].
    pub fn configure(&mut self, mut widget: NodeMut<'_>, id: WidgetId) {
        widget._configure(self, id);
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
    /// is desired or when [`SizeMgr::text_rules`] is used.
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

impl<'a> std::ops::BitOrAssign<Action> for ConfigMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: Action) {
        self.ev.send_action(action);
    }
}

impl<'a> Deref for ConfigMgr<'a> {
    type Target = EventState;
    fn deref(&self) -> &EventState {
        self.ev
    }
}
impl<'a> DerefMut for ConfigMgr<'a> {
    fn deref_mut(&mut self) -> &mut EventState {
        self.ev
    }
}
