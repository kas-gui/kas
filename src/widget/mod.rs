//! Widgets

#[cfg(not(feature = "layout"))]
#[macro_use] mod layout_extern;

#[cfg(feature = "cassowary")]
#[macro_use] mod layout_cw;

#[macro_use] mod make_widget;

mod class;
mod data;

#[cfg(not(feature = "layout"))]
pub use self::layout_extern::{Layout, ChildLayout, GridPos};
#[cfg(feature = "cassowary")]
pub use self::layout_cw::{Layout};

pub use self::data::*;
pub use self::class::Class;
use crate::toolkit::TkData;

/// Common widget behaviour
pub trait Core {
    /// Set the widget's number
    /// 
    /// This should only be called during widget enumeration
    fn set_number(&mut self, number: u32);
    
    /// Get the widget's number
    fn get_number(&self) -> u32;
    
    /// Get the toolkit data associated with this widget
    fn get_tkd(&self) -> TkData;
    
    /// Set the toolkit data associated with this widget
    fn set_tkd(&mut self, tkd: TkData);
    
    /// Get the widget's region, relative to its parent.
    fn rect(&self) -> &Rect;
    
    /// Get mutable access to the widget's region
    fn rect_mut(&mut self) -> &mut Rect;
}

/// Common widget data
/// 
/// Widgets should normally implement `Core` by use of an embedded field
/// of this type (i.e. composition). The `impl_core` macro may be used
/// to write the actual implementation:
/// 
/// ```
/// #[macro_use] extern crate mygui;
/// use mygui::widget;
/// 
/// struct MyWidget {
///     core: widget::CoreData,
///     // more fields here
/// }
/// 
/// impl_core!(MyWidget; core);
/// 
/// # fn main() {}
/// ```
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    number: u32,
    tkd: TkData,
    rect: Rect,
}

impl Core for CoreData {
    #[inline]
    fn set_number(&mut self, number: u32) {
        self.number = number;
    }
    #[inline]
    fn get_number(&self) -> u32 {
        self.number
    }
    
    #[inline]
    fn get_tkd(&self) -> TkData {
        self.tkd.clone()
    }
    
    #[inline]
    fn set_tkd(&mut self, tkd: TkData) {
        self.tkd = tkd;
    }
    
    #[inline]
    fn rect(&self) -> &Rect {
        &self.rect
    }
    
    #[inline]
    fn rect_mut(&mut self) -> &mut Rect {
        &mut self.rect
    }
}

/// A widget encapsulates code for event handling and/or drawing some feature
/// of a sub-region of a window.
/// 
/// Functionality common to all widgets is provided by the `Core` trait.
pub trait Widget: Layout {
    /// Get the widget's classification.
    fn class(&self) -> Class;
    
    /// Get the widget's label, if any.
    /// TODO: keep?
    fn label(&self) -> Option<&str>;
    
    /// Get the number of child widgets
    fn len(&self) -> usize;
    
    /// Get a reference to a child widget by index, or `None` if the index is
    /// out of bounds.
    /// 
    /// For convenience, `Index<usize>` is implemented via this method.
    /// 
    /// Required: `index < self.len()`.
    fn get(&self, index: usize) -> Option<&Widget>;
    
    /// Mutable variant of get
    fn get_mut(&mut self, index: usize) -> Option<&mut Widget>;
    
    /// Set the number for self and each child. Returns own number + 1.
    fn enumerate(&mut self, mut n: u32) -> u32 {
        for i in 0..self.len() {
            self.get_mut(i).map(|w| n = w.enumerate(n));
        }
        self.set_number(n);
        n + 1
    }
}

pub struct ChildIter<'a, W: 'a + Widget + ?Sized> {
    w: &'a W,
    i: ::std::ops::Range<usize>,
}

impl<'a, W: 'a + Widget + ?Sized> ChildIter<'a, W> {
    pub fn new(widget: &'a W) -> Self {
        ChildIter { w: widget, i: (0..widget.len()).into_iter() }
    }
}

impl<'a, W: 'a + Widget + ?Sized> Iterator for ChildIter<'a, W> {
    type Item = &'a Widget;
    fn next(&mut self) -> Option<Self::Item> {
        self.i.next().and_then(|i| self.w.get(i))
    }
}
/*
pub struct ChildIterMut<'a, W: 'static + Widget + ?Sized> {
    w: &'a mut W,
    i: ::std::ops::Range<usize>,
}

impl<'a, W: 'static + Widget + ?Sized> ChildIterMut<'a, W> {
    fn new(widget: &'a mut W) -> Self {
        let len = widget.len();
        ChildIterMut { w: widget, i: (0..len).into_iter() }
    }
    
    fn next<'b: 'a>(&'b mut self) -> Option<&'b mut Widget> {
        // TODO: resolve lifetime error (streaming iterator)
        self.i.next().and_then(|i| self.w.get_mut(i)).map(|w| &*w)
    }
}
*/
