//! Widgets

// TODO: use Deref instead? Would be more like inheritance.
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
pub mod event;
pub mod layout;
pub mod window;

use widget::layout::WidgetLayout;
use Rect;

/// Core widget trait (object-safe)
pub trait WidgetCore {
    /// Get the widget's region, relative to its parent.
    fn rect(&self) -> &Rect;
    
    /// Get mutable access to the widget's region
    fn rect_mut(&mut self) -> &mut Rect;
}

#[derive(Default)]
pub struct WidgetCoreData {
    rect: Rect,
}

impl WidgetCore for WidgetCoreData {
    fn rect(&self) -> &Rect {
        &self.rect
    }
    
    fn rect_mut(&mut self) -> &mut Rect {
        &mut self.rect
    }
}

/// Widget trait — includes user-customisable sub-type
pub trait Widget: WidgetLayout {
    type Response: From<event::NoResponse>;
    
    /// Handle an event, and return a user-defined message
    fn handle(&mut self, event: event::Event) -> Self::Response {
        event::ignore(event)
    }
}
