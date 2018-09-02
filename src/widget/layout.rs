//! Widget layout

use widget::WidgetCore;

pub trait WidgetLayout: WidgetCore {
    /// Minimum expected widget size; `(width, height)`
    fn min_size(&self) -> (i32, i32);
    
    /// Called at least once. Should position any sub-widgets.
    fn set_size(&mut self, size: (i32, i32)) {
        self.rect_mut().size = size;
    }
}

#[macro_export]
macro_rules! min_size_of {
    ($self:ident; none) => {
        (0, 0)
    };
    // TODO: syntactic ambiguity of layout name and sub-widgets!
    ($self:ident; vlist ( $g:ident ) ) => {
        $self.$g.min_size()
    };
    ($self:ident; vlist ( $g0:ident, $($g:ident),+ ) ) => {{
        let (mut w, mut h) = $self.$g0.min_size();
        $(
            let (w1, h1) = $self.$g.min_size();
            w = if w1 > w { w1 } else { w };
            h += h1;
        )+
        (w, h)
    }};
}

// TODO: this implementation naively assumes the size equals the minimum
#[macro_export]
macro_rules! set_size_of {
    ($self:ident; none; $w:expr, $h:expr) => {};
    ($self:ident; vlist ( $($g:ident),* ); $w:expr, $h:expr) => {
        $(
            // naive impl: common width, min heigth everywhere
            let (_, gh) = $self.$g.min_size();
            $self.$g.set_size(($w, gh));
        )*
    };
}

/// Implement the `WidgetLayout` trait for some type.
/// 
/// Usage:
/// ```nocompile
/// impl_layout!(MyWidget<T: WidgetLayout>; vlist(text, button));
/// ```
#[macro_export]
macro_rules! impl_layout {
    // this evil monstrosity matches <A, B: T, C: S+T>
    // but because there is no "zero or one" rule, also <D: S: T>
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >;
        $layout:ident $( $params:tt )* ) =>
    {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::layout::WidgetLayout
            for $ty< $( $N ),* >
        {
            fn min_size(&self) -> (i32, i32) {
                min_size_of!( self; $layout $($params)* )
            }

            fn set_size(&mut self, (w, h): (i32, i32)) {
                set_size_of!( self; $layout $($params)*; w, h );
            }
        }
    };
    ($ty:ident; $layout:ident $( $params:tt )* ) => {
        impl_layout!($ty<>; $layout $($params)*);
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
