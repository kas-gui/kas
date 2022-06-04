// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size and EventState manager

use crate::draw::DrawShared;
use crate::event::EventState;
use crate::geom::{Size, Vec2};
use crate::layout::Align;
use crate::text::TextApi;
use crate::theme::{SizeHandle, SizeMgr, TextClass};
use crate::{TkAction, Widget, WidgetId};
use std::ops::{Deref, DerefMut};

#[allow(unused)]
use crate::Layout;

/// Manager available to [`Layout::set_rect`] and [`Widget::configure`]
///
/// This type is functionally a superset of [`SizeMgr`] and subset of
/// [`crate::theme::DrawMgr`], with support for the appropriate conversions.
///
/// `SetRectMgr` supports [`Deref`] and [`DerefMut`] with target [`EventState`].
#[must_use]
pub struct SetRectMgr<'a> {
    sh: &'a dyn SizeHandle,
    ds: &'a mut dyn DrawShared,
    pub(crate) ev: &'a mut EventState,
}

impl<'a> SetRectMgr<'a> {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(sh: &'a dyn SizeHandle, ds: &'a mut dyn DrawShared, ev: &'a mut EventState) -> Self {
        SetRectMgr { sh, ds, ev }
    }

    /// Access a [`SizeMgr`]
    ///
    /// Warning: sizes are calculated using the window's current scale factor.
    /// This may change, even without user action, since some platforms
    /// always initialize windows with scale factor 1.
    /// See also notes on [`Widget::configure`].
    pub fn size_mgr(&self) -> SizeMgr<'a> {
        SizeMgr::new(self.sh)
    }

    /// Access [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.ds
    }

    /// Access [`EventState`]
    pub fn ev_state(&mut self) -> &mut EventState {
        self.ev
    }

    /// Configure a widget
    ///
    /// All widgets must be configured after construction (see
    /// [`Widget::configure`]). This method may be used to configure a new
    /// child widget without requiring the whole window to be reconfigured.
    ///
    /// Pass the `id` to assign to the widget: this should be constructed from
    /// the parent's id via [`WidgetId::make_child`].
    #[inline]
    pub fn configure(&mut self, id: WidgetId, widget: &mut dyn Widget) {
        widget.pre_configure(self, id);

        for index in 0..widget.num_children() {
            let id = widget.make_child_id(index);
            if let Some(widget) = widget.get_child_mut(index) {
                self.configure(id, widget);
            }
        }

        widget.configure(self);
    }

    /// Update a text object, setting font properties and wrap size
    ///
    /// Returns required size.
    #[inline]
    pub fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: (Align, Align),
    ) -> Vec2 {
        self.sh.text_set_size(text, class, size, align)
    }
}

impl<'a> std::ops::BitOrAssign<TkAction> for SetRectMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.ev.send_action(action);
    }
}

impl<'a> Deref for SetRectMgr<'a> {
    type Target = EventState;
    fn deref(&self) -> &EventState {
        self.ev
    }
}
impl<'a> DerefMut for SetRectMgr<'a> {
    fn deref_mut(&mut self) -> &mut EventState {
        self.ev
    }
}
