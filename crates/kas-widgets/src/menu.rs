// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menus

use kas::prelude::*;

mod menu_entry;
mod menubar;
mod submenu;

pub use menu_entry::{MenuEntry, MenuToggle};
pub use menubar::MenuBar;
pub use submenu::SubMenu;

/// Trait governing menus, sub-menus and menu-entries
#[autoimpl(for<T: trait + ?Sized> Box<T>)]
pub trait Menu: Widget {
    /// Report whether one's own menu is open
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

/// Provides a convenient `.boxed()` method on implementors
pub trait BoxedMenu<T: ?Sized> {
    /// Boxing method
    fn boxed_menu(self) -> Box<T>;
}

impl<M: Menu + Sized> BoxedMenu<dyn Menu<Msg = M::Msg>> for M {
    fn boxed_menu(self) -> Box<dyn Menu<Msg = M::Msg>> {
        Box::new(self)
    }
}
