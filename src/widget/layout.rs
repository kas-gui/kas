//! Layout widgets

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

impl<A: Widget, B: Widget> Widget for VList2<A, B> {}
