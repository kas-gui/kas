// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Cursor gallery

use kas::event::CursorIcon;
use kas::prelude::*;
use kas::widgets::{Column, Label, StrLabel};

impl_scope! {
    #[widget{
        Data = ();
        layout = self.label;
        cursor_icon = self.cursor;
    }]
    struct CursorWidget {
        core: widget_core!(),
        #[widget]
        label: StrLabel,
        cursor: CursorIcon,
    }
    impl Layout for Self {
        fn find_id(&mut self, coord: Coord) -> Option<Id> {
            // Steal mouse focus: hover points to self, not self.label
            self.rect().contains(coord).then(|| self.id())
        }
    }
}

// Using a macro lets us stringify! the type name
macro_rules! cursor {
    ($name: tt) => {
        CursorWidget {
            core: Default::default(),
            label: Label::new(stringify!($name)),
            cursor: CursorIcon::$name,
        }
    };
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    // These are winit::window::CursorIcon enum variants
    let column = Column::new([
        cursor!(Default),
        cursor!(ContextMenu),
        cursor!(Help),
        cursor!(Pointer),
        cursor!(Progress),
        cursor!(Wait),
        cursor!(Cell),
        cursor!(Crosshair),
        cursor!(Text),
        cursor!(VerticalText),
        cursor!(Alias),
        cursor!(Copy),
        cursor!(Move),
        cursor!(NoDrop),
        cursor!(NotAllowed),
        cursor!(Grab),
        cursor!(Grabbing),
        cursor!(EResize),
        cursor!(NResize),
        cursor!(NeResize),
        cursor!(NwResize),
        cursor!(SResize),
        cursor!(SeResize),
        cursor!(SwResize),
        cursor!(WResize),
        cursor!(EwResize),
        cursor!(NsResize),
        cursor!(NeswResize),
        cursor!(NwseResize),
        cursor!(ColResize),
        cursor!(RowResize),
        cursor!(AllScroll),
        cursor!(ZoomIn),
        cursor!(ZoomOut),
    ]);

    let window = Window::new(column, "Cursor gallery");
    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new((), theme)?.with(window).run()
}
