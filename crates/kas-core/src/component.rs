// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget components

use crate::geom::{Coord, Rect, Size};
use crate::layout::{Align, AlignHints, AxisInfo, SetRectMgr, SizeRules};
use crate::text::{format, AccelString, Text, TextApi};
use crate::theme::{DrawMgr, IdCoord, SizeMgr, TextClass};
use crate::{TkAction, WidgetId};
use kas_macros::{autoimpl, impl_scope};

/// Components are not true widgets, but support layout solving
///
/// TODO: since this is a sub-set of widget functionality, should [`crate::Widget`]
/// extend `Component`? (Significant trait revision would be required.)
pub trait Component {
    /// Get size rules for the given axis
    fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules;

    /// Apply a given `rect` to self
    fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints);

    /// True if the layout direction is up/left (reverse reading direction)
    ///
    /// TODO: replace with spatial_nav?
    fn is_reversed(&self) -> bool {
        false
    }

    /// Translate a coordinate to a [`WidgetId`]
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId>;

    /// Draw the component and its children
    fn draw(&mut self, draw: DrawMgr, id: &WidgetId);
}

impl_scope! {
    /// A label component
    #[impl_default(where T: trait)]
    #[autoimpl(Clone, Debug where T: trait)]
    pub struct Label<T: format::FormattableText> {
        text: Text<T>,
        class: TextClass = TextClass::Label(false),
        pos: Coord,
    }

    impl Self {
        /// Construct
        #[inline]
        pub fn new(label: T, class: TextClass) -> Self {
            Label {
                text: Text::new_single(label),
                class,
                pos: Default::default(),
            }
        }

        /// Get text
        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }

        /// Set the text and prepare
        ///
        /// Update text and trigger a resize if necessary.
        ///
        /// The `avail` parameter is used to determine when a resize is required. If
        /// this parameter is a little bit wrong then resizes may sometimes happen
        /// unnecessarily or may not happen when text is slightly too big (e.g.
        /// spills into the margin area); this behaviour is probably acceptable.
        /// Passing `Size::ZERO` will always resize (unless text is empty).
        /// Passing `Size::MAX` should never resize.
        pub fn set_text_and_prepare(&mut self, s: T, avail: Size) -> TkAction {
            self.text.set_text(s);
            crate::text::util::prepare_if_needed(&mut self.text, avail)
        }

        /// Set the text from a string and prepare
        ///
        /// Update text and trigger a resize if necessary.
        ///
        /// The `avail` parameter is used to determine when a resize is required. If
        /// this parameter is a little bit wrong then resizes may sometimes happen
        /// unnecessarily or may not happen when text is slightly too big (e.g.
        /// spills into the margin area); this behaviour is probably acceptable.
        pub fn set_string_and_prepare(&mut self, s: String, avail: Size) -> TkAction
        where
            T: format::EditableText,
        {
            use crate::text::EditableTextApi;
            self.text.set_string(s);
            crate::text::util::prepare_if_needed(&mut self.text, avail)
        }

        /// Get class
        pub fn class(&self) -> TextClass {
            self.class
        }

        /// Set class
        ///
        /// This may influence layout.
        pub fn set_class(&mut self, class: TextClass) -> TkAction {
            self.class = class;
            TkAction::RESIZE
        }
    }

    impl Label<AccelString> {
        /// Get the accelerator keys
        pub fn keys(&self) -> &[crate::event::VirtualKeyCode] {
            self.text.text().keys()
        }
    }

    impl Component for Self {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            mgr.text_bound(&mut self.text, self.class, axis)
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            self.pos = rect.pos;
            let halign = match self.class {
                TextClass::Button => Align::Center,
                _ => Align::Default,
            };
            let align = align.unwrap_or(halign, Align::Center);
            mgr.text_set_size(&mut self.text, self.class, rect.size, align);
        }

        fn find_id(&mut self, _: Coord) -> Option<WidgetId> {
            None
        }

        fn draw(&mut self, mut draw: DrawMgr, id: &WidgetId) {
            draw.text_effects(IdCoord(id, self.pos), &self.text, self.class);
        }
    }
}
