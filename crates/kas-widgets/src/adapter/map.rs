// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Message Map widget

use crate::menu::{self, Menu};
use kas::prelude::*;
use std::fmt::Debug;
use std::marker::PhantomData;

impl_scope! {
    /// Wrapper to map messages from the inner widget
    #[autoimpl(Debug ignore self.map)]
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone)]
    #[widget{
        layout = self.inner;
    }]
    pub struct MapMessage<W: Widget, M: Debug + 'static, N: Debug + 'static, F: FnMut(M) -> N + 'static> {
        #[widget_core]
        core: kas::CoreData,
        #[widget]
        inner: W,
        map: F,
        _m: PhantomData<M>,
        _n: PhantomData<N>,
    }

    impl Self {
        /// Construct
        ///
        /// Any response from the child widget with a message payload is mapped
        /// through the closure `map`.
        pub fn new(inner: W, map: F) -> Self {
            MapMessage {
                core: Default::default(),
                inner,
                map,
                _m: PhantomData,
                _n: PhantomData,
            }
        }
    }

    impl Widget for Self {
        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(msg) = mgr.try_pop_msg() {
                mgr.push_msg((self.map)(msg));
            }
        }
    }

    impl Menu for Self where W: Menu {
        fn sub_items(&mut self) -> Option<menu::SubItems> {
            self.inner.sub_items()
        }
        fn menu_is_open(&self) -> bool {
            self.inner.menu_is_open()
        }
        fn set_menu_path(&mut self, mgr: &mut EventMgr, target: Option<&WidgetId>, set_focus: bool) {
            self.inner.set_menu_path(mgr, target, set_focus);
        }
    }
}
