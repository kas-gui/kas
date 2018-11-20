//! Dialogs
//! 
//! NOTE: this functionality is broken and will likely be altered significantly.

use std::fmt::{self, Debug};

use crate::control::{button, TextButton};
use crate::macros::Widget;
use crate::event::{Action, GuiResponse};
use crate::{Class, CoreData, TkWidget, Widget, Window};


/// An action for use with `MessageBox::new`.
pub fn action_close() -> impl Fn() -> GuiResponse {
    || GuiResponse::Close
}

/// A message box.
#[layout]
#[widget(class = Class::Window)]
#[derive(Clone, Widget)]
pub struct MessageBox<M: Debug, H> {
    #[core] core: CoreData,
    message: M,
    button: TextButton<H>,
}

impl<M: Debug, R, H: Fn() -> R> MessageBox<M, H> {
    // TODO: action parameter shouldn't be necessary, but we need it because
    // H must be derived from function input somehow, not merely unspecified
    // Once existential types are available, H parameter will not be needed.
    pub fn new(message: M, action: H) -> Self {
        MessageBox{
            core: Default::default(),
            message,
            button: button::ok(action)
        }
    }
}

// manual impl required because derive requires `H: Debug`
impl<M: Debug, H> Debug for MessageBox<M, H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MessageBox {{ core: {:?}, message: {:?}, button: {:?} }}",
            self.core, self.message, self.button)
    }
}

impl<M: Debug, H> Window for MessageBox<M, H> {
    fn as_widget(&self) -> &Widget { self }
    fn as_widget_mut(&mut self) -> &mut Widget { self }
    
    #[cfg(feature = "layout")]
    fn configure_widgets(&mut self, _tk: &TkWidget) {
        unimplemented!()
    }
    
    #[cfg(feature = "layout")]
    fn resize(&mut self, _tk: &TkWidget, _size: Coord) {
        unimplemented!()
    }
    
    fn handle_action(&mut self, _tk: &TkWidget, _action: Action, _num: u32) -> GuiResponse
    {
        unimplemented!()
    }
}
