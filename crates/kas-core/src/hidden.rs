// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Hidden extras
//!
//! It turns out that some widgets are needed in kas-core. This module is
//! hidden by default and direct usage (outside of kas crates) is
//! not supported (i.e. **changes are not considered breaking**).

use crate::class::HasStr;
use crate::event::{ConfigMgr, Event, EventMgr, Response};
use crate::geom::{Coord, Offset, Rect};
use crate::layout::{Align, AxisInfo, SizeRules};
use crate::text::{AccelString, Text, TextApi};
use crate::theme::{DrawMgr, SizeMgr, TextClass};
use crate::{Erased, Layout, NavAdvance, Node, NodeMut, Widget, WidgetCore, WidgetId};
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
        label: Text<&'static str>,
    }

    impl Self {
        /// Construct from `label`
        #[inline]
        pub fn new(label: &'static str) -> Self {
            StrLabel {
                core: Default::default(),
                label: Text::new(label),
            }
        }

        /// Text class
        pub const CLASS: TextClass = TextClass::Label(false);
    }

    impl Layout for Self {
        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, mut axis: AxisInfo) -> SizeRules {
            axis.set_default_align_hv(Align::Default, Align::Center);
            size_mgr.text_rules(&mut self.label, Self::CLASS, axis)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;
            mgr.text_set_size(&mut self.label, Self::CLASS, rect.size, None);
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.text(self.rect(), &self.label, Self::CLASS);
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }
}

impl_scope! {
    /// Map any input data to `()`
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    pub struct WithAny<A, W: Widget<Data = ()>> {
        _a: std::marker::PhantomData<A>,
        pub inner: W,
    }

    impl Self {
        /// Construct
        pub fn new(inner: W) -> Self {
            WithAny {
                _a: std::marker::PhantomData,
                inner,
            }
        }
    }

    // We don't use #[widget] here. This is not supported outside of Kas!
    impl WidgetCore for Self {
        #[inline]
        fn id_ref(&self) -> &WidgetId {
            self.inner.id_ref()
        }

        #[inline]
        fn rect(&self) -> Rect {
            self.inner.rect()
        }

        #[inline]
        fn widget_name(&self) -> &'static str {
            "WithAny"
        }
    }

    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.inner.num_children()
        }

        #[inline]
        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            self.inner.find_child_index(id)
        }

        #[inline]
        fn make_child_id(&mut self, index: usize) -> WidgetId {
            self.inner.make_child_id(index)
        }

        #[inline]
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.inner.size_rules(size_mgr, axis)
        }

        #[inline]
        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.inner.set_rect(mgr, rect);
        }

        #[inline]
        fn nav_next(&self, reverse: bool, from: Option<usize>) -> Option<usize> {
            self.inner.nav_next(reverse, from)
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.inner.translation()
        }

        #[inline]
        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.inner.find_id(coord)
        }

        #[inline]
        fn draw(&mut self, draw: DrawMgr) {
            self.inner.draw(draw);
        }
    }

    impl Widget for Self {
        type Data = A;

        fn as_node(&self, _: &A) -> Node<'_> {
            self.inner.as_node(&())
        }
        fn as_node_mut(&mut self, _: &A) -> NodeMut<'_> {
            self.inner.as_node_mut(&())
        }

        fn get_child(&self, _: &A, index: usize) -> Option<Node<'_>> {
            self.inner.get_child(&(), index)
        }
        fn get_child_mut(&mut self, _: &A, index: usize) -> Option<NodeMut<'_>> {
            self.inner.get_child_mut(&(), index)
        }

        fn _configure(&mut self, _: &A, cx: &mut ConfigMgr, id: WidgetId) {
            self.inner._configure(&(), cx, id);
        }

        fn _broadcast(&mut self, _: &A, cx: &mut EventMgr, count: &mut usize, event: Event) {
            self.inner._broadcast(&(), cx, count, event);
        }

        fn _send(&mut self, _: &A, cx: &mut EventMgr, id: WidgetId, disabled: bool, event: Event) -> Response {
            self.inner._send(&(), cx, id, disabled, event)
        }

        fn _replay(&mut self, _: &A, cx: &mut EventMgr, id: WidgetId, msg: Erased) {
            self.inner._replay(&(), cx, id, msg);
        }

        fn _nav_next(
            &mut self,
            _: &A,
            cx: &mut EventMgr,
            focus: Option<&WidgetId>,
            advance: NavAdvance,
        ) -> Option<WidgetId> {
            self.inner._nav_next(&(), cx, focus, advance)
        }
    }
}
