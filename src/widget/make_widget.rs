//! The impl_widget and make_widget macros

#[macro_export]
macro_rules! count_items {
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + $crate::count_items!($($rest),*)
    }
}

/// Implements `WidgetCore` using a field of type `CoreData`
#[macro_export]
macro_rules! impl_widget_core {
    // This matches bounds like: <A, B: T, C: S+T>
    // TODO: use RFC 2298 when stable for `: BOUND` part
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >; $core:ident) => {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::WidgetCore
            for $ty< $( $N ),* >
        {
            fn set_number(&mut self, number: u32) {
                self.$core.set_number(number);
            }
            
            fn get_number(&self) -> u32 {
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
    ($ty:ident; $core:ident) => {
        $crate::impl_widget_core!($ty<>; $core);
    };
}

/// Implements `Widget`
#[macro_export]
macro_rules! impl_widget {
    // this evil monstrosity matches <A, B: T, C: S+T>
    // TODO: use RFC 2298 when stable for `: BOUND` part
    ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )* ),* >;
        $class:path; $label:expr; $($wname:ident),*) =>
    {
        impl< $( $N $(: $b0 $(+$b)* )* ),* >
            $crate::widget::Widget
            for $ty< $( $N ),* >
        {
            fn class(&self) -> $crate::widget::Class { $class }
            fn label(&self) -> Option<&str> { $label }

            fn len(&self) -> usize {
                $crate::count_items!($($wname),*)
            }
            fn get(&self, index: usize) -> Option<&$crate::widget::Widget> {
                // We need to match, but macros cannot expand to match arms
                // or parts of if-else chains. Hack: use direct return.
                let _i = 0;
                $(
                    if index == _i {
                        return Some(&self.$wname);
                    }
                    let _i = _i + 1;
                )*
                return None;
            }
            fn get_mut(&mut self, index: usize) -> Option<&mut $crate::widget::Widget> {
                let _i = 0;
                $(
                    if index == _i {
                        return Some(&mut self.$wname);
                    }
                    let _i = _i + 1;
                )*
                return None;
            }
        }
    };
    ($ty:ident; $class:path; $label:expr; $($wname:ident),*) => {
        $crate::impl_widget!($ty<>; $class; $label; $($wname),*);
    };
}

/// Construct a container widget
#[macro_export]
macro_rules! make_widget {
    // Full version, allowing custom event handlers per child widget
    ($direction:ident < $($gt:ident [$gtr:path]),* >;
        $self:ident, $tk: ident, $msg:ident;
        $(
            $([$($pos:expr),*])*    // zero or one times (TODO RFC 2298)
            $wname:ident: $wt:ident = $wvalue:expr
            => $whandle:expr
        ),* ;
        $($dname:ident: $dt:ident = $dvalue:expr),* ;
        $response:path) =>
    {{
        use $crate::event::{Action, Handler, ignore};
        use $crate::toolkit::Toolkit;
        use $crate::widget::{Class, CoreData, WidgetCore, Widget};

        #[derive(Clone, Debug)]
        struct L<$($gt: Widget + 'static),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        $crate::impl_widget_core!(L<$($gt: Widget),*>; core);
        $crate::impl_widget_layout!(L<$($gt: Widget),*>; 
            $direction; $( $( [ $($pos),* ] )* $wname),*);
        $crate::impl_widget!(L<$($gt: Widget),*>; Class::Container; None; $($wname),*);

        impl<$($gt: Widget + Handler<Response = $gtr>),*> Handler
            for L<$($gt),*>
        {
            type Response = $response;
            
            fn handle_action(&mut $self, $tk: &Toolkit, action: Action, num: u32) -> $response {
                $(
                    if num <= $self.$wname.get_number() {
                        let $msg = $self.$wname.handle_action($tk, action, num);
                        return $whandle;
                    }
                )*
                if num == $self.get_number() {
                    ignore(action)  // no actions handled by this widget
                } else {
                    println!("Warning: incorrect widget number");
                    ignore(action)
                }
            }
        }

        L {
            core: Default::default(),
            $($wname: $wvalue),* ,
            $($dname: $dvalue),*
        }
    }};
    // Simplified version with only pass-through event handling
    ($direction:ident < $($gt:ident $gtr:ident),* >;
        $(
            $([$($pos:expr),*])*    // zero or one times (TODO RFC 2298)
            $wname:ident: $wt:ident = $wvalue:expr
        ),* ;
        $($dname:ident: $dt:ident = $dvalue:expr),* ;
        $response:path) =>
    {{
        use $crate::event::{Action, Handler, ignore};
        use $crate::toolkit::Toolkit;
        use $crate::widget::{Class, CoreData, WidgetCore, Widget};

        #[derive(Clone, Debug)]
        struct L<$($gt: Widget + 'static),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        $crate::impl_widget_core!(L<$($gt: Widget),*>; core);
        $crate::impl_widget_layout!(L<$($gt: Widget),*>; 
            $direction; $( $( [ $($pos),* ] )* $wname),*);
        $crate::impl_widget!(L<$($gt: Widget),*>; Class::Container; None; $($wname),*);

        impl<$($gtr, $gt: Widget + Handler<Response = $gtr>),*> Handler
            for L<$($gt),*>
            where $($gtr: From<NoResponse>, $response: From<$gtr>),*
        {
            type Response = $response;
            
            fn handle_action(&mut self, tk: &Toolkit, action: Action, num: u32) -> $response {
                $(
                    if num <= self.$wname.get_number() {
                        return self.$wname.handle_action(tk, action, num).into();
                    }
                )*
                if num == self.get_number() {
                    ignore(action)  // no actions handled by this widget
                } else {
                    println!("Warning: incorrect widget number");
                    ignore(action)
                }
            }
        }

        L {
            core: Default::default(),
            $($wname: $wvalue),* ,
            $($dname: $dvalue),*
        }
    }}
}

#[cfg(test)] mod test {
    use crate::widget::{Widget, Layout, WidgetCore, canvas::Text};
    use crate::event::NoResponse;

    #[test]
    #[allow(unused)] // spurious warning in unit test
    fn macro_test_layout() {
        fn check_props<T: Widget + Layout + WidgetCore>(_x: T) {}
        
        let w = make_widget!(single<>; self, tk, msg;
            text: Text = Text::from("text") => msg;;
            NoResponse);
        check_props(w);
    }
}
