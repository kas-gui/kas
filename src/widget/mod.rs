//! Widgets

/// Implements `WidgetCore` using a field of type `CoreData`
#[macro_export]
macro_rules! impl_widget_core {
    // this evil monstrosity matches <A, B: T, C: S+T>
    // but because there is no "zero or one" rule, also <D: S: T>
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >, $core:ident) => {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::WidgetCore
            for $ty< $( $N ),* >
        {
            fn set_number(&mut self, number: usize) {
                self.$core.set_number(number);
            }
            
            fn get_number(&self) -> usize {
                self.$core.get_number()
            }
            
            fn get_tkd(&self) -> $crate::toolkit::TkData {
                self.$core.get_tkd()
            }
            fn set_tkd(&mut self, tkd: $crate::toolkit::TkData) {
                self.$core.set_tkd(tkd)
            }
            
            fn rect(&self) -> &$crate::Rect {
                self.$core.rect()
            }
            fn rect_mut(&mut self) -> &mut $crate::Rect {
                self.$core.rect_mut()
            }
        }
    };
    ($ty:ident, $core:ident) => {
        impl_widget_core!($ty<>, $core);
    };
}

#[macro_use] mod layout;

pub mod canvas;
pub mod control;
pub mod window;

mod class;

pub use self::class::Class;
pub use self::layout::Layout;
use Rect;
use toolkit::TkData;

/// Common widget behaviour
pub trait WidgetCore {
    /// Set the widget's number
    /// 
    /// This should only be called during widget enumeration
    fn set_number(&mut self, number: usize);
    
    /// Get the widget's number
    fn get_number(&self) -> usize;
    
    /// Get the toolkit data associated with this widget
    fn get_tkd(&self) -> TkData;
    
    /// Set the toolkit data associated with this widget
    fn set_tkd(&mut self, tkd: TkData);
    
    /// Get the widget's region, relative to its parent.
    fn rect(&self) -> &Rect;
    
    /// Get mutable access to the widget's region
    fn rect_mut(&mut self) -> &mut Rect;
}

impl<'a, W: WidgetCore> WidgetCore for &'a mut W {
    fn set_number(&mut self, number: usize) { (**self).set_number(number) }
    fn get_number(&self) -> usize { (**self).get_number() }
    
    fn get_tkd(&self) -> TkData { (**self).get_tkd() }
    fn set_tkd(&mut self, tkd: TkData) { (**self).set_tkd(tkd) }
    
    fn rect(&self) -> &Rect { (**self).rect() }
    fn rect_mut(&mut self) -> &mut Rect { (**self).rect_mut() }
}

/// Common widget data
/// 
/// Widgets should normally implement `WidgetCore` by use of an embedded field
/// of this type (i.e. composition). The `impl_widget_core` macro may be used
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
/// impl_widget_core!(MyWidget, core);
/// 
/// # fn main() {}
/// ```
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    number: usize,
    tkd: TkData,
    rect: Rect,
}

impl WidgetCore for CoreData {
    #[inline]
    fn set_number(&mut self, number: usize) {
        self.number = number;
    }
    #[inline]
    fn get_number(&self) -> usize {
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
/// Functionality common to all widgets is provided by the `WidgetCore` trait.
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
    fn enumerate(&mut self, mut n: usize) -> usize {
        for i in 0..self.len() {
            self.get_mut(i).map(|w| n = w.enumerate(n));
        }
        self.set_number(n);
        n + 1
    }
}

impl<'a, W: Widget> Widget for &'a mut W {
    fn class(&self) -> Class { (**self).class() }
    fn label(&self) -> Option<&str> { (**self).label() }
    
    fn len(&self) -> usize { (**self).len() }
    fn get(&self, index: usize) -> Option<&Widget> { (**self).get(index) }
    fn get_mut(&mut self, index: usize) -> Option<&mut Widget> {
        (**self).get_mut(index)
    }
    
    fn enumerate(&mut self, mut n: usize) -> usize {
        (**self).enumerate(n)
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
