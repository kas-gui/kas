//! Layout widgets

use widget::event;
use super::Widget;

/// Vertical list of two widgets
pub struct VList2<A: Widget, B: Widget> {
    a: A,
    b: B
}

impl<A: Widget, B: Widget> VList2<A, B> {
    pub fn new(a: A, b: B) -> Self {
        VList2 { a, b }
    }
}

impl<A: Widget, B: Widget> Widget for VList2<A, B> {
    type Response = event::NoResponse;    //TODO: pass to children
}

#[macro_export]
macro_rules! impl_layout {
    ($ty:ty; $layout:ident; $( $widget:ident ),* , ) => {   // trailing comma
        impl_layout!($ty; $layout; $( $widget ),* )
    };
    ($ty:ty; $layout:ident; $( $widget:ident ),* ) => {
        //TODO
    };
}

#[macro_export]
macro_rules! match_event_widget {
    ($ev:expr; $( $name:ident => $result:expr ),* , ) => {  // trailing comma
        match_event_widget!($ev; $( $name => $result ),* )
    };
    ($ev:expr; $name0:ident => $result0:expr, $( $name:ident => $result:expr ),* ) => {
        // TODO: find correct widget; currently we just assume the first
        $result0
    };
}
