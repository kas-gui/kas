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
    pub label: Option<&'a mut dyn Layout>,
    /// Secondary label, often used to show shortcut key
    pub label2: Option<&'a mut dyn Layout>,
    /// Sub-menu indicator
    pub submenu: Option<&'a mut dyn Layout>,
    /// Icon
    pub icon: Option<&'a mut dyn Layout>,
    /// Toggle mark
    pub toggle: Option<&'a mut dyn Layout>,
}

/// Trait governing menus, sub-menus and menu-entries
///
/// Implementations will automatically receive nav focus on mouse-hover, thus
/// should ensure that [`Layout::find_id`] returns the identifier of the widget
/// which should be focussed, and that this widget has
/// [`Widget::navigable`] return true.
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Menu: Widget {
    /// Access row items for aligned layout
    ///
    /// If this returns sub-items, then these items are aligned in the menu view. This involves
    /// (1) calling `Self::size_rules` and `Self::set_rect` like usual, and (2) running an external
    /// layout solver on these items (which also calls `size_rules` and `set_rect` on each item).
    /// This is redundant, but ensures the expectations on [`Layout::size_rules`] and
    /// [`Layout::set_rect`] are met.
    ///
    /// Note further: if this returns `Some(_)`, then spacing for menu item frames is added
    /// "magically" by the caller. The implementor should draw a frame as follows:
    /// ```
    /// # use kas::geom::Rect;
    /// # use kas::theme::{DrawMgr, FrameStyle};
    /// # struct S;
    /// # impl S {
    /// # fn rect(&self) -> Rect { Rect::ZERO }
    /// fn draw(&mut self, mut draw: DrawMgr) {
    ///     draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
    ///     // draw children here
    /// }
    /// # }
    /// ```
    // TODO: adding frame spacing like this is quite hacky. Find a better approach?
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
    fn set_menu_path(&mut self, mgr: &mut EventCx<()>, target: Option<&WidgetId>, set_focus: bool) {
        let _ = (mgr, target, set_focus);
    }
}

/// A boxed menu
pub type BoxedMenu = Box<dyn Menu<Data = ()>>;

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
        F: Fn(&mut EventCx<()>, bool) + 'static,
    {
        self.menu
            .push(Box::new(MenuToggle::new(label).on_toggle(f)));
    }

    /// Append a [`MenuToggle`], chain style
    #[inline]
    pub fn toggle<S: Into<AccelString>, F>(mut self, label: S, f: F) -> Self
    where
        F: Fn(&mut EventCx<()>, bool) + 'static,
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
