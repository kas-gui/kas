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

pub mod canvas;
pub mod control;
pub mod window;

mod class;
mod layout;

pub use self::class::Class;
pub use self::layout::Layout;
use Rect;

/// Common widget behaviour
pub trait WidgetCore {
    /// Get the widget's region, relative to its parent.
    fn rect(&self) -> &Rect;
    
    /// Get mutable access to the widget's region
    fn rect_mut(&mut self) -> &mut Rect;
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
#[derive(Clone, Default)]
pub struct CoreData {
    rect: Rect,
}

impl WidgetCore for CoreData {
    fn rect(&self) -> &Rect {
        &self.rect
    }
    
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
    fn get(&self, index: usize) -> Option<&(dyn Widget + 'static)>;
}

impl ::std::ops::Index<usize> for dyn Widget + 'static {
    type Output = dyn Widget + 'static;
    fn index(&self, i: usize) -> &Self::Output {
        self.get(i).unwrap_or_else(|| panic!("Widget::get: index out of bounds"))
    }
}
