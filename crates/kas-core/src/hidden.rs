// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hidden extras
//!
//! It turns out that some widgets are needed in kas-core. This module is
//! hidden by default and direct usage (outside of kas crates) is
//! not supported (i.e. **changes are not considered breaking**).

use crate::classes::HasStr;
use crate::event::{ConfigCx, Event, EventCx, IsUsed};
use crate::geom::Rect;
use crate::layout::{Align, AlignHints, AxisInfo, SizeRules};
use crate::theme::{DrawCx, SizeCx, Text, TextClass};
use crate::{Events, Id, Layout, NavAdvance, Node, Widget};
use kas_macros::{autoimpl, impl_scope};

impl_scope! {
    /// A simple text label
    ///
    /// Vertical alignment defaults to centred, horizontal
    /// alignment depends on the script direction if not specified.
    /// Line-wrapping is enabled.
    #[derive(Clone, Debug, Default)]
    #[widget {
        Data = ();
    }]
    pub struct StrLabel {
        core: widget_core!(),
        text: Text<&'static str>,
    }

    impl Self {
        /// Construct from `text`
        #[inline]
        pub fn new(text: &'static str) -> Self {
            StrLabel {
                core: Default::default(),
                text: Text::new(text, TextClass::Label(false)),
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            sizer.text_rules(&mut self.text, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.core.rect = rect;
            let align = hints.complete(Align::Default, Align::Center);
            cx.text_set_size(&mut self.text, rect.size, align);
        }

        fn draw(&mut self, mut draw: DrawCx) {
            draw.text(self.rect(), &self.text);
        }
    }

    impl Events for Self {
        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.text_configure(&mut self.text);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.text.as_str()
        }
    }
}

impl_scope! {
    /// Map any input data to `()`
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    pub struct MapAny<A, W: Widget<Data = ()>> {
        _a: std::marker::PhantomData<A>,
        pub inner: W,
    }

    impl Self {
        /// Construct
        pub fn new(inner: W) -> Self {
            MapAny {
                _a: std::marker::PhantomData,
                inner,
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn as_layout(&self) -> &dyn ::kas::Layout {
            self
        }
        #[inline]
        fn id_ref(&self) -> &::kas::Id {
            self.inner.id_ref()
        }
        #[inline]
        fn rect(&self) -> ::kas::geom::Rect {
            self.inner.rect()
        }

        #[inline]
        fn widget_name(&self) -> &'static str {
            "MapAny"
        }

        #[inline]
        fn num_children(&self) -> usize {
            self.inner.num_children()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn ::kas::Layout> {
            self.inner.get_child(index)
        }
        #[inline]
        fn find_child_index(&self, id: &::kas::Id) -> Option<usize> {
            self.inner.find_child_index(id)
        }

        #[inline]
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            self.inner.size_rules(sizer, axis)
        }

        #[inline]
        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            self.inner.set_rect(cx, rect, hints);
        }

        #[inline]
        fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
            self.inner.nav_next(reverse, from)
        }

        #[inline]
        fn translation(&self) -> ::kas::geom::Offset {
            self.inner.translation()
        }

        // NOTE: fn probe is left unimplemented since it should not be called directly

        #[inline]
        fn try_probe(&mut self, coord: ::kas::geom::Coord) -> Option<::kas::Id> {
            self.inner.try_probe(coord)
        }

        #[inline]
        fn draw(&mut self, draw: DrawCx) {
            self.inner.draw(draw);
        }
    }

    impl Widget for Self {
        type Data = A;

        fn as_node<'a>(&'a mut self, _: &'a A) -> Node<'a> {
            self.inner.as_node(&())
        }

        #[inline]
        fn for_child_node(
            &mut self,
            _: &A,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            self.inner.for_child_node(&(), index, closure)
        }

        fn _configure(&mut self, cx: &mut ConfigCx, _: &A, id: Id) {
            self.inner._configure(cx, &(), id);
        }

        fn _update(&mut self, cx: &mut ConfigCx, _: &A) {
            self.inner._update(cx, &());
        }

        fn _send(&mut self, cx: &mut EventCx, _: &A, id: Id, event: Event) -> IsUsed {
            self.inner._send(cx, &(), id, event)
        }

        fn _replay(&mut self, cx: &mut EventCx, _: &A, id: Id) {
            self.inner._replay(cx, &(), id);
        }

        fn _nav_next(
            &mut self,
            cx: &mut ConfigCx,
            _: &A,
            focus: Option<&Id>,
            advance: NavAdvance,
        ) -> Option<Id> {
            self.inner._nav_next(cx, &(), focus, advance)
        }
    }
}
