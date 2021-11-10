// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Message Map widget

use kas::prelude::*;
use std::fmt;
use std::rc::Rc;

widget! {
    /// Wrapper to map messages from the inner widget
    #[derive(Clone)]
    #[layout(single)]
    #[handler(msg=M)]
    pub struct MapResponse<W: Widget, M: 'static> {
        #[widget_core]
        core: kas::CoreData,
        #[widget]
        inner: W,
        map: Rc<dyn Fn(&mut Manager, W::Msg) -> Response<M>>,
    }

    impl Self {
        /// Construct
        ///
        /// Any response from the child widget with a message payload is mapped
        /// through the closure `f`.
        pub fn new<F: Fn(&mut Manager, W::Msg) -> Response<M> + 'static>(child: W, f: F) -> Self {
            Self::new_rc(child, Rc::new(f))
        }

        /// Construct with an Rc-wrapped method
        ///
        /// Any response from the child widget with a message payload is mapped
        /// through the closure `f`.
        pub fn new_rc(child: W, f: Rc<dyn Fn(&mut Manager, W::Msg) -> Response<M>>) -> Self {
            MapResponse {
                core: Default::default(),
                inner: child,
                map: f,
            }
        }
    }

    impl fmt::Debug for Self {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.debug_struct("MapResponse")
                .field("core", &self.core)
                .field("inner", &self.inner)
                .finish_non_exhaustive()
        }
    }

    impl SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
            if self.is_disabled() {
                return Response::Unhandled;
            }

            if id < self.id() {
                let r = self.inner.send(mgr, id, event);
                r.try_into().unwrap_or_else(|msg| {
                    log::trace!(
                        "Received by {} from {}: {:?}",
                        self.id(),
                        id,
                        kas::util::TryFormat(&msg)
                    );
                    (self.map)(mgr, msg)
                })
            } else {
                debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
                self.handle(mgr, event)
            }
        }
    }

    impl HasBool for Self
    where
        W: HasBool,
    {
        fn get_bool(&self) -> bool {
            self.inner.get_bool()
        }

        fn set_bool(&mut self, state: bool) -> TkAction {
            self.inner.set_bool(state)
        }
    }

    impl HasStr for Self
    where
        W: HasStr,
    {
        fn get_str(&self) -> &str {
            self.inner.get_str()
        }
    }

    impl HasString for Self
    where
        W: HasString,
    {
        fn set_string(&mut self, text: String) -> TkAction {
            self.inner.set_string(text)
        }
    }

    // TODO: HasFormatted

    impl SetAccel for Self
    where
        W: SetAccel,
    {
        fn set_accel_string(&mut self, accel: AccelString) -> TkAction {
            self.inner.set_accel_string(accel)
        }
    }

    impl std::ops::Deref for Self {
        type Target = W;
        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl std::ops::DerefMut for Self {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }
}
