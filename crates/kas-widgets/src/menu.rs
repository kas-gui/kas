// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menus

use crate::adapter::AdaptWidget;
use crate::Separator;
use kas::component::Component;
use kas::dir::Right;
use kas::prelude::*;
use std::fmt::Debug;

mod menu_entry;
mod menubar;
mod submenu;

pub use menu_entry::{MenuEntry, MenuToggle};
pub use menubar::{MenuBar, MenuBuilder};
pub use submenu::SubMenu;

/// Trait governing menus, sub-menus and menu-entries
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Menu: Widget {
    /// Access row items for aligned layout
    ///
    /// If this is implemented, the row will be sized and layout through direct
    /// access to these sub-components. [`Layout::size_rules`] will not be
    /// invoked on `self`. [`Layout::set_rect`] will be, but should not set the
    /// position of these items. [`Layout::draw`] should draw all components,
    /// including a frame with style [`kas::theme::FrameStyle::MenuEntry`] on
    /// self.
    ///
    /// Return value is `None` or `Some((label, opt_label2, opt_submenu, opt_icon, opt_toggle))`.
    /// `opt_label2` is used to show shortcut labels. `opt_submenu` is a sub-menu indicator.
    fn menu_sub_items(
        &mut self,
    ) -> Option<(
        &mut dyn Component,
        Option<&mut dyn Component>,
        Option<&mut dyn Component>,
        Option<&mut dyn Component>,
        Option<&mut dyn WidgetConfig>,
    )> {
        None
    }

    /// Report whether a submenu (if any) is open
    ///
    /// By default, this is `false`.
    fn menu_is_open(&self) -> bool {
        false
    }

    /// Open or close a sub-menu, including parents
    ///
    /// Given `Some(id) = target`, the sub-menu with this `id` should open its
    /// menu; if it has child-menus, these should close; and if any ancestors
    /// are menus, these should open.
    ///
    /// `target == None` implies that all menus should close.
    ///
    /// When opening menus and `set_focus` is true, the first navigable child
    /// of the newly opened menu will be given focus. This is used for keyboard
    /// navigation only.
    fn set_menu_path(&mut self, mgr: &mut EventMgr, target: Option<&WidgetId>, set_focus: bool) {
        let _ = (mgr, target, set_focus);
    }
}

/// A boxed menu
pub type BoxedMenu<M> = Box<dyn Menu<Msg = M>>;

/// Builder for a [`SubMenu`]
///
/// Access through [`MenuBar::builder`].
pub struct SubMenuBuilder<'a, M: 'static> {
    menu: &'a mut Vec<BoxedMenu<M>>,
}

impl<'a, M: 'static> SubMenuBuilder<'a, M> {
    /// Append an item
    #[inline]
    pub fn push_item(&mut self, item: BoxedMenu<M>) {
        self.menu.push(item);
    }

    /// Append an item, chain style
    #[inline]
    pub fn item(mut self, item: BoxedMenu<M>) -> Self {
        self.push_item(item);
        self
    }

    /// Append a [`MenuEntry`]
    pub fn push_entry<S: Into<AccelString>>(&mut self, label: S, msg: M)
    where
        M: Clone + Debug,
    {
        self.menu.push(Box::new(MenuEntry::new(label, msg)));
    }

    /// Append a [`MenuEntry`], chain style
    #[inline]
    pub fn entry<S: Into<AccelString>>(mut self, label: S, msg: M) -> Self
    where
        M: Clone + Debug,
    {
        self.push_entry(label, msg);
        self
    }

    /// Append a [`MenuToggle`]
    pub fn push_toggle<S: Into<AccelString>, F>(&mut self, label: S, f: F)
    where
        F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
    {
        self.menu
            .push(Box::new(MenuToggle::new(label).on_toggle(f)));
    }

    /// Append a [`MenuToggle`], chain style
    #[inline]
    pub fn toggle<S: Into<AccelString>, F>(mut self, label: S, f: F) -> Self
    where
        F: Fn(&mut EventMgr, bool) -> Option<M> + 'static,
    {
        self.push_toggle(label, f);
        self
    }

    /// Append a [`Separator`]
    pub fn push_separator(&mut self) {
        self.menu.push(Box::new(Separator::new().map_void_msg()));
    }

    /// Append a [`Separator`], chain style
    #[inline]
    pub fn separator(mut self) -> Self {
        self.push_separator();
        self
    }

    /// Append a [`SubMenu`]
    ///
    /// This submenu prefers opens to the right.
    #[inline]
    pub fn push_submenu<F>(&mut self, label: impl Into<AccelString>, f: F)
    where
        F: FnOnce(SubMenuBuilder<M>),
    {
        self.push_submenu_with_dir(Right, label, f);
    }

    /// Append a [`SubMenu`], chain style
    ///
    /// This submenu prefers opens to the right.
    #[inline]
    pub fn submenu<F>(mut self, label: impl Into<AccelString>, f: F) -> Self
    where
        F: FnOnce(SubMenuBuilder<M>),
    {
        self.push_submenu_with_dir(Right, label, f);
        self
    }

    /// Append a [`SubMenu`]
    ///
    /// This submenu prefers to open in the specified direction.
    pub fn push_submenu_with_dir<D, F>(&mut self, dir: D, label: impl Into<AccelString>, f: F)
    where
        D: Directional,
        F: FnOnce(SubMenuBuilder<M>),
    {
        let mut menu = Vec::new();
        f(SubMenuBuilder { menu: &mut menu });
        self.menu
            .push(Box::new(SubMenu::new_with_direction(dir, label, menu)));
    }

    /// Append a [`SubMenu`], chain style
    ///
    /// This submenu prefers to open in the specified direction.
    #[inline]
    pub fn submenu_with_dir<D, F>(mut self, dir: D, label: impl Into<AccelString>, f: F) -> Self
    where
        D: Directional,
        F: FnOnce(SubMenuBuilder<M>),
    {
        self.push_submenu_with_dir(dir, label, f);
        self
    }
}
