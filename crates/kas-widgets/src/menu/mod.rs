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

use crate::adapt::MapAny;
use crate::Separator;
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
    pub label: Option<&'a mut dyn Tile>,
    /// Secondary label, often used to show shortcut key
    pub label2: Option<&'a mut dyn Tile>,
    /// Sub-menu indicator
    pub submenu: Option<&'a mut dyn Tile>,
    /// Icon
    pub icon: Option<&'a mut dyn Tile>,
    /// Toggle mark
    pub toggle: Option<&'a mut dyn Tile>,
}

/// Trait governing menus, sub-menus and menu-entries
///
/// Implementations will automatically receive nav focus on mouse-hover, thus
/// should ensure that [`Tile::probe`] returns the identifier of the widget
/// which should be focussed, and that this widget has
/// [`Events::navigable`] return true.
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
    /// # use kas::theme::{DrawCx, FrameStyle};
    /// # struct S;
    /// # impl S {
    /// # fn rect(&self) -> Rect { Rect::ZERO }
    /// fn draw(&self, mut draw: DrawCx) {
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
    fn set_menu_path(
        &mut self,
        cx: &mut EventCx,
        data: &Self::Data,
        target: Option<&Id>,
        set_focus: bool,
    ) {
        let _ = (cx, data, target, set_focus);
    }
}

impl<A, W: Menu<Data = ()>> Menu for MapAny<A, W> {
    fn sub_items(&mut self) -> Option<SubItems> {
        self.inner.sub_items()
    }

    fn menu_is_open(&self) -> bool {
        self.inner.menu_is_open()
    }

    fn set_menu_path(&mut self, cx: &mut EventCx, _: &A, target: Option<&Id>, set_focus: bool) {
        self.inner.set_menu_path(cx, &(), target, set_focus);
    }
}

/// A boxed menu
pub type BoxedMenu<Data> = Box<dyn Menu<Data = Data>>;

/// Builder for a [`SubMenu`]
///
/// Access through [`MenuBar::builder`].
pub struct SubMenuBuilder<'a, Data> {
    menu: &'a mut Vec<BoxedMenu<Data>>,
}

impl<'a, Data> SubMenuBuilder<'a, Data> {
    /// Append an item
    #[inline]
    pub fn push_item(&mut self, item: BoxedMenu<Data>) {
        self.menu.push(item);
    }

    /// Append an item, chain style
    #[inline]
    pub fn item(mut self, item: BoxedMenu<Data>) -> Self {
        self.push_item(item);
        self
    }
}

impl<'a, Data: 'static> SubMenuBuilder<'a, Data> {
    /// Append a [`MenuEntry`]
    pub fn push_entry<S: Into<AccessString>, M>(&mut self, label: S, msg: M)
    where
        M: Clone + Debug + 'static,
    {
        self.menu
            .push(Box::new(MapAny::new(MenuEntry::new_msg(label, msg))));
    }

    /// Append a [`MenuEntry`], chain style
    #[inline]
    pub fn entry<S: Into<AccessString>, M>(mut self, label: S, msg: M) -> Self
    where
        M: Clone + Debug + 'static,
    {
        self.push_entry(label, msg);
        self
    }

    /// Append a [`MenuToggle`]
    pub fn push_toggle<M: Debug + 'static>(
        &mut self,
        label: impl Into<AccessString>,
        state_fn: impl Fn(&ConfigCx, &Data) -> bool + 'static,
        msg_fn: impl Fn(bool) -> M + 'static,
    ) {
        self.menu
            .push(Box::new(MenuToggle::new_msg(label, state_fn, msg_fn)));
    }

    /// Append a [`MenuToggle`], chain style
    pub fn toggle<M: Debug + 'static>(
        mut self,
        label: impl Into<AccessString>,
        state_fn: impl Fn(&ConfigCx, &Data) -> bool + 'static,
        msg_fn: impl Fn(bool) -> M + 'static,
    ) -> Self {
        self.push_toggle(label, state_fn, msg_fn);
        self
    }

    /// Append a [`Separator`]
    pub fn push_separator(&mut self) {
        self.menu.push(Box::new(MapAny::new(Separator::new())));
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
    pub fn push_submenu<F>(&mut self, label: impl Into<AccessString>, f: F)
    where
        F: FnOnce(SubMenuBuilder<Data>),
    {
        self.push_submenu_dir(label, f, Direction::Right);
    }

    /// Append a [`SubMenu`], chain style
    ///
    /// This submenu prefers opens to the right.
    pub fn submenu<F>(mut self, label: impl Into<AccessString>, f: F) -> Self
    where
        F: FnOnce(SubMenuBuilder<Data>),
    {
        self.push_submenu(label, f);
        self
    }

    /// Append a [`SubMenu`]
    ///
    /// This submenu prefers to open in the specified direction.
    pub fn push_submenu_dir<F>(&mut self, label: impl Into<AccessString>, f: F, dir: Direction)
    where
        F: FnOnce(SubMenuBuilder<Data>),
    {
        let mut menu = Vec::new();
        f(SubMenuBuilder { menu: &mut menu });
        self.menu.push(Box::new(SubMenu::new(label, menu, dir)));
    }

    /// Append a [`SubMenu`], chain style
    ///
    /// This submenu prefers to open in the specified direction.
    #[inline]
    pub fn submenu_dir<F>(mut self, label: impl Into<AccessString>, f: F, dir: Direction) -> Self
    where
        F: FnOnce(SubMenuBuilder<Data>),
    {
        self.push_submenu_dir(label, f, dir);
        self
    }
}
