//! Widget layout

use Coord;
use widget::WidgetCore;
use toolkit::Toolkit;

pub trait Layout: WidgetCore {
    /// Get the widget's default (preferred) size.
    /// 
    /// The default implementation simply refers to the toolkit — for toolkit
    /// native widgets this correct.
    // TODO: more flexible if instead this adds constraints?
    fn default_size(&self, tk: &Toolkit) -> Coord {
        tk.tk_widget().default_size(self.get_tkd())
    }
    
    /// Called at least once. Should position any sub-widgets.
    fn set_size(&mut self, size: (i32, i32)) {
        self.rect_mut().size = size;
    }
}

// #[macro_export]
// macro_rules! min_size_of {
//     ($self:ident; $direction:ident;) => {
//         (0, 0)
//     };
//     ($self:ident; $direction:ident; $g:ident) => {
//         $self.$g.min_size()
//     };
//     ($self:ident; vertical; $g0:ident, $($g:ident),+) => {{
//         let (mut w, mut h) = $self.$g0.min_size();
//         $(
//             let (w1, h1) = $self.$g.min_size();
//             w = if w1 > w { w1 } else { w };
//             h += h1;
//         )+
//         (w, h)
//     }};
// }

// TODO: this implementation naively assumes the size equals the minimum
#[macro_export]
macro_rules! set_size_of {
    ($self:ident; $direction:ident; ; $w:expr, $h:expr) => {};
    ($self:ident; vertical; $($g:ident),*; $w:expr, $h:expr) => {
        unimplemented!()
//         $(
//             // naive impl: common width, min heigth everywhere
//             let (_, gh) = $self.$g.min_size();
//             $self.$g.set_size(($w, gh));
//         )*
    };
}

#[macro_export]
macro_rules! count_items {
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + count_items!($($rest),*)
    }
}

#[macro_export]
macro_rules! iter_get_widget {
    ($self:ident, $index:ident, $i:expr ) => { return None; };
    ($self:ident, $index:ident, $i:expr, $name:ident) => {
        if $index == $i {
            return Some(&$self.$name);
        }
        return None;
    };
    ($self:ident, $index:ident, $i:expr, $name:ident, $($wname:ident),*) => {
        if $index == $i {
            return Some(&$self.$name);
        }
        iter_get_widget!($self, $index, ($i + 1), $($wname),*);
    }
}

#[macro_export]
macro_rules! iter_get_widget_mut {
    ($self:ident, $index:ident, $i:expr ) => { return None; };
    ($self:ident, $index:ident, $i:expr, $name:ident) => {
        if $index == $i {
            return Some(&mut $self.$name);
        }
        return None;
    };
    ($self:ident, $index:ident, $i:expr, $name:ident, $($wname:ident),*) => {
        if $index == $i {
            return Some(&mut $self.$name);
        }
        iter_get_widget_mut!($self, $index, ($i + 1), $($wname),*);
    }
}

/// Construct a container widget
#[macro_export]
macro_rules! make_layout {
    ($direction:ident;
        $($wname:ident $wt:ident : $wvalue:expr),* ;
        $($dname:ident $dt:ident : $dvalue:expr),* ;) =>
    {{
        use $crate::widget::{Class, CoreData, Widget, Layout};
        use $crate::event::{Event, Handler, Response};

        struct L<$($wt: Widget + 'static),* , $($dt),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        impl_widget_core!(L<$($wt: Widget),* , $($dt),*>, core);

        impl<$($wt: Widget),* , $($dt),*> Layout
            for L<$($wt),* , $($dt),*>
        {
//             fn min_size(&self) -> (i32, i32) {
//                 min_size_of!(self; $direction; $($wname),*)
//             }

            fn set_size(&mut self, size: (i32, i32)) {
                set_size_of!(self; $direction; $($wname),*; size.0, size.1);
            }
        }

        impl<$($wt: Widget + 'static),* , $($dt),*> Widget
            for L<$($wt),* , $($dt),*>
        {
            fn class(&self) -> Class { Class::Container }
            fn label(&self) -> Option<&str> { None }

            fn len(&self) -> usize {
                count_items!($($wname),*)
            }
            fn get(&self, index: usize) -> Option<&Widget> {
                // We need to match, but macros cannot expand to match arms
                // or parts of if-else chains. Hack: use direct return.
                iter_get_widget!(self, index, 0, $($wname),*);
            }
            fn get_mut(&mut self, index: usize) -> Option<&mut Widget> {
                iter_get_widget_mut!(self, index, 0, $($wname),*);
            }
        }

        impl<$($wt: Widget),* , $($dt),*> Handler
            for L<$($wt),* , $($dt),*>
        {
            type Response = Response;   // TODO
            fn handle(&mut self, event: Event) -> Self::Response {
                unimplemented!()
            }
        }

        L {
            core: Default::default(),
            $($wname: $wvalue),* ,
            $($dname: $dvalue),*
        }
    }}
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
