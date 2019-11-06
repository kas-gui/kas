// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.

use std::fmt::{self, Debug};

use crate::event::{Callback, Event, Handler, Response};
use crate::geom::Size;
use crate::macros::Widget;
use crate::widget::{button, TextButton};
use crate::{class::Class, CoreData, TkWindow, Widget, Window};

/// An action for use with `MessageBox::new`.
pub fn action_close() -> impl Fn() -> Response<()> {
    || Response::Msg(()) // TODO: close
}

/// A message box. TODO: this needs revision.
#[widget(class = Class::Window, layout = derive)]
#[derive(Clone, Widget)]
pub struct MessageBox<M: Debug + 'static, H: 'static> {
    #[core]
    core: CoreData,
    message: M,
    button: TextButton<H>,
}

impl<M: Debug, R, H: Fn() -> R> MessageBox<M, H> {
    // TODO: action parameter shouldn't be necessary, but we need it because
    // H must be derived from function input somehow, not merely unspecified
    // Once existential types are available, H parameter will not be needed.
    pub fn new(message: M, action: H) -> Self {
        MessageBox {
            core: Default::default(),
            message,
            button: button::ok(action),
        }
    }
}

// manual impl required because derive requires `H: Debug`
impl<M: Debug, H> Debug for MessageBox<M, H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "MessageBox {{ core: {:?}, message: {:?}, button: {:?} }}",
            self.core, self.message, self.button
        )
    }
}

impl<M: Debug, H> Handler for MessageBox<M, H> {
    type Msg = ();

    fn handle(&mut self, _tk: &mut dyn TkWindow, _event: Event) -> Response<Self::Msg> {
        unimplemented!()
    }
}

impl<M: Debug, H> Window for MessageBox<M, H> {
    fn as_widget(&self) -> &dyn Widget {
        self
    }
    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self
    }

    fn configure(&mut self) {
        unimplemented!()
    }

    fn resize(&mut self, _tk: &mut dyn TkWindow, _size: Size) {
        unimplemented!()
    }

    // doesn't support callbacks, so doesn't need to do anything here
    fn callbacks(&self) -> Vec<(usize, Callback)> {
        Vec::new()
    }
    fn trigger_callback(&mut self, _index: usize, _tk: &mut dyn TkWindow) {}
}
