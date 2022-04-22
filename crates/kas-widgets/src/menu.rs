// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu widgets
//!
//! The following serve as menu roots:
//!
//! -   [`crate::ComboBox`]
//! -   [`MenuBar`]
//!
//! Any implementation of the [`Menu`] trait may be used as a menu item:
//!
//! -   [`SubMenu`]
//! -   [`MenuEntry`]
//! -   [`MenuToggle`]
//! -   [`Separator`]

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

/// Return value of [`Menu::sub_items`]
#[derive(Default)]
pub struct SubItems<'a> {
    /// Primary label
    pub label: Option<&'a mut dyn Component>,
    /// Secondary label, often used to show shortcut key
    pub label2: Option<&'a mut dyn Component>,
    /// Sub-menu indicator
    pub submenu: Option<&'a mut dyn Component>,
    /// Icon
    pub icon: Option<&'a mut dyn Component>,
    /// Toggle mark
    // TODO: should be a component?
    pub toggle: Option<&'a mut dyn WidgetConfig>,
}

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
    fn sub_items(&mut self) -> Option<SubItems> {
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
pub type BoxedMenu = Box<dyn Menu>;

/// Builder for a [`SubMenu`]
///
/// Access through [`MenuBar::builder`].
pub struct SubMenuBuilder<'a> {
    menu: &'a mut Vec<BoxedMenu>,
}

impl<'a> SubMenuBuilder<'a> {
    /// Append an item
    #[inline]
    pub fn push_item(&mut self, item: BoxedMenu) {
        self.menu.push(item);
    }

    /// Append an item, chain style
    #[inline]
    pub fn item(mut self, item: BoxedMenu) -> Self {
        self.push_item(item);
        self
    }

    /// Append a [`MenuEntry`]
    pub fn push_entry<S: Into<AccelString>, M>(&mut self, label: S, msg: M)
    where
        M: Clone + Debug + 'static,
    {
        self.menu.push(Box::new(MenuEntry::new(label, msg)));
    }

    /// Append a [`MenuEntry`], chain style
    #[inline]
    pub fn entry<S: Into<AccelString>, M>(mut self, label: S, msg: M) -> Self
    where
        M: Clone + Debug + 'static,
    {
        self.push_entry(label, msg);
        self
    }

    /// Append a [`MenuToggle`]
    pub fn push_toggle<S: Into<AccelString>, F>(&mut self, label: S, f: F)
    where
        F: Fn(&mut EventMgr, bool) + 'static,
    {
        self.menu
            .push(Box::new(MenuToggle::new(label).on_toggle(f)));
    }

    /// Append a [`MenuToggle`], chain style
    #[inline]
    pub fn toggle<S: Into<AccelString>, F>(mut self, label: S, f: F) -> Self
    where
        F: Fn(&mut EventMgr, bool) + 'static,
    {
        self.push_toggle(label, f);
        self
    }

    /// Append a [`Separator`]
    pub fn push_separator(&mut self) {
        self.menu.push(Box::new(Separator::new()));
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
        F: FnOnce(SubMenuBuilder),
    {
        self.push_submenu_with_dir(Right, label, f);
    }

    /// Append a [`SubMenu`], chain style
    ///
    /// This submenu prefers opens to the right.
    #[inline]
    pub fn submenu<F>(mut self, label: impl Into<AccelString>, f: F) -> Self
    where
        F: FnOnce(SubMenuBuilder),
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
        F: FnOnce(SubMenuBuilder),
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
        F: FnOnce(SubMenuBuilder),
    {
        self.push_submenu_with_dir(dir, label, f);
        self
    }
}
