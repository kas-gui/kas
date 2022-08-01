// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Size and EventState manager

use super::Pending;
use crate::draw::DrawShared;
use crate::event::EventState;
use crate::geom::{Rect, Size};
use crate::layout::{Align, AlignHints};
use crate::text::TextApi;
use crate::theme::{Feature, SizeMgr, TextClass, ThemeSize};
use crate::{TkAction, Widget, WidgetExt, WidgetId};
use std::ops::{Deref, DerefMut};

#[allow(unused)]
use crate::{event::Event, Layout};

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

    /// Align a feature's rect
    ///
    /// In case the input `rect` is larger than desired on either axis, it is
    /// reduced in size and offset within the original `rect` as is preferred.
    #[inline]
    pub fn align_feature(&self, feature: Feature, rect: Rect, hints: AlignHints) -> Rect {
        self.sh.align_feature(feature, rect, hints)
    }

    /// Prepare a text object
    ///
    /// This sets the text's font, font size, wrapping and alignment and
    /// performs text preparation necessary before display.
    #[inline]
    pub fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: (Align, Align),
    ) {
        self.sh.text_set_size(text, class, size, align)
    }

    /// Advance the keyboard navigation focus
    ///
    /// If some widget currently has nav focus, this will give focus to the next
    /// (or previous) widget under `widget` where [`Widget::key_nav`]
    /// returns true; otherwise this will give focus to the first (or last)
    /// such widget.
    ///
    /// Returns true on success, false if there are no navigable widgets or
    /// some error occurred.
    ///
    /// The target widget will receive [`Event::NavFocus`] with `key_focus` as
    /// the payload. This boolean should be true if focussing in response to
    /// keyboard input, false if reacting to mouse or touch input.
    pub fn next_nav_focus(
        &mut self,
        mut widget: &mut dyn Widget,
        reverse: bool,
        key_focus: bool,
    ) -> bool {
        if let Some(id) = self.popups.last().map(|(_, p, _)| p.id.clone()) {
            if id.is_ancestor_of(widget.id_ref()) {
                // do nothing
            } else if let Some(w) = widget.find_widget_mut(&id) {
                widget = w;
            } else {
                log::warn!(
                    target: "kas_core::event::config_mgr",
                    "next_nav_focus: have open pop-up which is not a child of widget",
                );
                return false;
            }
        }

        // We redraw in all cases. Since this is not part of widget event
        // processing, we can push directly to self.action.
        self.send_action(TkAction::REDRAW);
        let old_nav_focus = self.nav_focus.take();

        fn nav(
            mgr: &mut ConfigMgr,
            widget: &mut dyn Widget,
            focus: Option<&WidgetId>,
            rev: bool,
        ) -> Option<WidgetId> {
            if mgr.ev_state().is_disabled(widget.id_ref()) {
                return None;
            }

            let mut child = focus.and_then(|id| widget.find_child_index(id));

            if !rev {
                if let Some(index) = child {
                    if let Some(id) = widget
                        .get_child_mut(index)
                        .and_then(|w| nav(mgr, w, focus, rev))
                    {
                        return Some(id);
                    }
                } else if !widget.eq_id(focus) && widget.key_nav() {
                    return Some(widget.id());
                }

                loop {
                    if let Some(index) = widget.spatial_nav(mgr, rev, child) {
                        if let Some(id) = widget
                            .get_child_mut(index)
                            .and_then(|w| nav(mgr, w, focus, rev))
                        {
                            return Some(id);
                        }
                        child = Some(index);
                    } else {
                        return None;
                    }
                }
            } else {
                if let Some(index) = child {
                    if let Some(id) = widget
                        .get_child_mut(index)
                        .and_then(|w| nav(mgr, w, focus, rev))
                    {
                        return Some(id);
                    }
                }

                loop {
                    if let Some(index) = widget.spatial_nav(mgr, rev, child) {
                        if let Some(id) = widget
                            .get_child_mut(index)
                            .and_then(|w| nav(mgr, w, focus, rev))
                        {
                            return Some(id);
                        }
                        child = Some(index);
                    } else {
                        return if !widget.eq_id(focus) && widget.key_nav() {
                            Some(widget.id())
                        } else {
                            None
                        };
                    }
                }
            }
        }

        // Whether to restart from the beginning on failure
        let restart = self.nav_focus.is_some();

        let mut opt_id = nav(self, widget, old_nav_focus.as_ref(), reverse);
        if restart && opt_id.is_none() {
            opt_id = nav(self, widget, None, reverse);
        }

        log::trace!(
            target: "kas_core::event::config_mgr",
            "next_nav_focus: nav_focus={opt_id:?}",
        );
        self.nav_focus = opt_id.clone();

        if opt_id == old_nav_focus {
            return opt_id.is_some();
        }

        if let Some(id) = old_nav_focus {
            self.pending.push_back(Pending::LostNavFocus(id));
        }

        if let Some(id) = opt_id {
            if id != self.sel_focus {
                self.clear_char_focus();
            }
            self.pending.push_back(Pending::SetNavFocus(id, key_focus));
            true
        } else {
            // Most likely an error occurred
            self.clear_char_focus();
            false
        }
    }
}

impl<'a> std::ops::BitOrAssign<TkAction> for ConfigMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
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
