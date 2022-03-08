// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Message Map widget

use kas::prelude::*;
use std::rc::Rc;

widget! {
    /// Wrapper to map messages from the inner widget
    #[autoimpl(Debug ignore self.map)]
    #[autoimpl(Deref, DerefMut on self.inner)]
    #[autoimpl(class_traits where W: trait on self.inner)]
    #[derive(Clone)]
    #[widget{
        layout = single;
    }]
    #[handler(msg=M)]
    pub struct MapResponse<W: Widget, M: 'static> {
        #[widget_core]
        core: kas::CoreData,
        #[widget]
        inner: W,
        map: Rc<dyn Fn(&mut EventMgr, W::Msg) -> Response<M>>,
    }

    impl Self {
        /// Construct
        ///
        /// Any response from the child widget with a message payload is mapped
        /// through the closure `f`.
        pub fn new<F: Fn(&mut EventMgr, W::Msg) -> Response<M> + 'static>(child: W, f: F) -> Self {
            Self::new_rc(child, Rc::new(f))
        }

        /// Construct with an Rc-wrapped method
        ///
        /// Any response from the child widget with a message payload is mapped
        /// through the closure `f`.
        pub fn new_rc(child: W, f: Rc<dyn Fn(&mut EventMgr, W::Msg) -> Response<M>>) -> Self {
            MapResponse {
                core: Default::default(),
                inner: child,
                map: f,
            }
        }
    }

    impl SendEvent for Self {
        fn send(&mut self, mgr: &mut EventMgr, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.eq_id(&id) {
                self.handle(mgr, event)
            } else {
                let r = self.inner.send(mgr, id.clone(), event);
                r.try_into().unwrap_or_else(|msg| {
                    log::trace!(
                        "Received by {} from {}: {:?}",
                        self.id(),
                        id,
                        kas::util::TryFormat(&msg)
                    );
                    (self.map)(mgr, msg)
                })
            }
        }
    }
}
