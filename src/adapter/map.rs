// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Message Map widget

use crate::prelude::*;
use std::fmt;
use std::rc::Rc;

/// Wrapper to map messages from the inner widget
#[derive(Clone, Widget)]
#[layout(single)]
#[handler(msg=M, send=noauto)]
pub struct MapResponse<W: Widget, M: 'static> {
    #[widget_core]
    core: kas::CoreData,
    #[widget]
    inner: W,
    map: Rc<dyn Fn(&mut Manager, W::Msg) -> Response<M>>,
}

impl<W: Widget, M> MapResponse<W, M> {
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

impl<W: Widget, M> fmt::Debug for MapResponse<W, M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MapResponse")
            .field("core", &self.core)
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<W: Widget, M> SendEvent for MapResponse<W, M> {
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

impl<W: Widget + HasBool, M> HasBool for MapResponse<W, M> {
    fn get_bool(&self) -> bool {
        self.inner.get_bool()
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.inner.set_bool(state)
    }
}

impl<W: Widget + HasStr, M> HasStr for MapResponse<W, M> {
    fn get_str(&self) -> &str {
        self.inner.get_str()
    }
}

impl<W: Widget + HasString, M> HasString for MapResponse<W, M> {
    fn set_string(&mut self, text: String) -> TkAction {
        self.inner.set_string(text)
    }
}

// TODO: HasFormatted

impl<W: Widget + SetAccel, M> SetAccel for MapResponse<W, M> {
    fn set_accel_string(&mut self, accel: AccelString) -> TkAction {
        self.inner.set_accel_string(accel)
    }
}

impl<W: Widget, M> std::ops::Deref for MapResponse<W, M> {
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W: Widget, M> std::ops::DerefMut for MapResponse<W, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
