//! Window widgets

use std::fmt::{self, Debug};

use crate::widget::Coord;
use crate::event::{self, Action, Handler, ignore};
use crate::widget::{Class, Layout, Widget, CoreData, WidgetCore};
use crate::control::{button, TextButton};
use crate::toolkit::Toolkit;

/// A window is a drawable interactive region provided by windowing system.
// TODO: should this be a trait, instead of simply a struct? Should it be
// implemented by dialogs?
pub trait Window: Widget {
    /// Upcast
    /// 
    /// Note: needed because Rust does not yet support trait object upcasting
    fn as_widget(&self) -> &Widget;
    /// Upcast, mutably
    /// 
    /// Note: needed because Rust does not yet support trait object upcasting
    fn as_widget_mut(&mut self) -> &mut Widget;
    
    /// Calculate and update positions for all sub-widgets
    #[cfg(feature = "layout")]
    fn configure_widgets(&mut self, tk: &Toolkit);
    
    /// Adjust the size of the window, repositioning widgets.
    /// 
    /// `configure_widgets` must be called before this.
    #[cfg(feature = "layout")]
    fn resize(&mut self, tk: &Toolkit, size: Coord);
    
    /// Handle a high-level event directed at the widget identified by `num`,
    /// and return a user-defined message.
    // NOTE: we could instead add the trait bound Handler<Response = Response>
    // but (1) Rust doesn't yet support mult-trait objects
    // and (2) Rust erronously claims that Response isn't specified in Box<Window>
    fn handle_action(&mut self, tk: &Toolkit, action: Action, num: u32) -> Response;
}

/// Window event repsonses
pub enum Response {
    /// No action
    None,
    /// Close the window
    Close,
}

impl From<event::NoResponse> for Response {
    fn from(r: event::NoResponse) -> Self {
        match r {
            event::NoResponse::None => Response::None
        }
    }
}

/// Main window type
pub struct SimpleWindow<W> {
    core: CoreData,
    min_size: Coord,
    #[cfg(feature = "cassowary")] solver: crate::cw::Solver,
    w: W
}

impl<W: Debug> Debug for SimpleWindow<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SimpleWindow {{ core: {:?}, min_size: {:?}, solver: <omitted>, w: {:?} }}",
            self.core, self.min_size, self.w)
    }
}

impl<W: Clone> Clone for SimpleWindow<W> {
    fn clone(&self) -> Self {
        SimpleWindow {
            core: self.core.clone(),
            min_size: self.min_size,
            #[cfg(feature = "cassowary")] solver: crate::cw::Solver::new(),
            w: self.w.clone()
        }
    }
}

impl_widget_core!(SimpleWindow<W>; core);
impl_layout_single!(SimpleWindow<W: Layout + Debug>, w);
impl_widget!(SimpleWindow<W: Widget>; Class::Window; None; w);

impl<W: Widget> SimpleWindow<W> {
    /// Create
    pub fn new(w: W) -> SimpleWindow<W> {
        SimpleWindow {
            core: Default::default(),
            min_size: (0, 0),
            #[cfg(feature = "cassowary")] solver: crate::cw::Solver::new(),
            w
        }
    }
}

impl<R, W: Widget + Handler<Response = R> + 'static> Window
    for SimpleWindow<W>
    where Response: From<R>, R: From<event::NoResponse>
{
    fn as_widget(&self) -> &Widget { self }
    fn as_widget_mut(&mut self) -> &mut Widget { self }
    
    #[cfg(feature = "cassowary")]
    fn configure_widgets(&mut self, tk: &Toolkit) {
        use crate::cw;
        assert!(self.get_number() > 0, "widget not enumerated");
        
        let v_w = cw_var!(self, w);
        let v_h = cw_var!(self, h);
        
        self.solver.reset();
        
        self.w.init_constraints(tk.tk_widget(), &mut self.solver, true);
        
        self.solver.add_edit_variable(v_w, cw::strength::MEDIUM * 100.0).unwrap();
        self.solver.add_edit_variable(v_h, cw::strength::MEDIUM * 100.0).unwrap();
        
        self.min_size = (self.solver.get_value(v_w) as i32, self.solver.get_value(v_h) as i32);
        
        self.w.apply_constraints(tk.tk_widget(), &self.solver, (0, 0));
    }
    
    #[cfg(feature = "cassowary")]
    fn resize(&mut self, tk: &Toolkit, size: Coord) {
        assert!(self.get_number() > 0, "widget not enumerated");
        
        self.solver.suggest_value(cw_var!(self, w), size.0 as f64).unwrap();
        self.solver.suggest_value(cw_var!(self, h), size.1 as f64).unwrap();
        
        self.w.apply_constraints(tk.tk_widget(), &self.solver, (0, 0));
    }
    
    fn handle_action(&mut self, tk: &Toolkit, action: Action, num: u32) -> Response {
        if num < self.get_number() {
            Response::from(self.w.handle_action(tk, action, num))
        } else if num == self.get_number() {
            match action {
                Action::Close => Response::Close,
                _ => ignore(action)
            }
        } else {
            println!("Warning: incorrect widget number");
            ignore(action)
        }
    }
}


pub fn action_close() -> impl Fn() -> Response {
    || Response::Close
}

#[derive(Clone)]
pub struct MessageBox<M, H> {
    core: CoreData,
    message: M,
    button: TextButton<H>,
}

impl<M, R, H: Fn() -> R> MessageBox<M, H> {
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

impl_widget_core!(MessageBox<M, H>; core);
impl_layout_single!(MessageBox<M: Debug, H>, button);  // TODO: improve?

impl<M: Debug, H> Widget for MessageBox<M, H> {
    fn class(&self) -> Class { Class::Window }
    fn label(&self) -> Option<&str> { None }
    
    fn len(&self) -> usize { 0 }
    fn get(&self, _index: usize) -> Option<&Widget> {
        unimplemented!()
    }
    fn get_mut(&mut self, _index: usize) -> Option<&mut Widget> {
        unimplemented!()
    }
}

impl<M: Debug, H> Window for MessageBox<M, H> {
    fn as_widget(&self) -> &Widget { self }
    fn as_widget_mut(&mut self) -> &mut Widget { self }
    
    #[cfg(feature = "layout")]
    fn configure_widgets(&mut self, _tk: &Toolkit) {
        unimplemented!()
    }
    
    #[cfg(feature = "layout")]
    fn resize(&mut self, _tk: &Toolkit, _size: Coord) {
        unimplemented!()
    }
    
    fn handle_action(&mut self, _tk: &Toolkit, _action: Action, _num: u32) -> Response {
        unimplemented!()
    }
}
