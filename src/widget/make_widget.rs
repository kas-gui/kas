//! The make_widget macro

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
        use $crate::widget::{Class, Core, CoreData, Widget};

        #[$crate::mygui_impl(Core(core),
            Widget(class=Class::Container, children=[$($wname),*]))]
        #[derive(Clone, Debug)]
        struct L<$($gt: Widget + 'static),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        $crate::impl_layout!(L<$($gt: Widget),*>; 
            $direction; $( $( [ $($pos),* ] )* $wname),*);

        impl<$($gt: Widget + Handler<Response = $gtr>),*> Handler
            for L<$($gt),*>
        {
            type Response = $response;
            
            fn handle_action(&mut $self, $tk: &Toolkit, action: Action, num: u32) -> $response {
                $(
                    if num <= $self.$wname.number() {
                        let $msg = $self.$wname.handle_action($tk, action, num);
                        return $whandle;
                    }
                )*
                if num == $self.number() {
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
        use $crate::widget::{Class, Core, CoreData, Widget};

        #[$crate::mygui_impl(Core(core),
            Widget(class=Class::Container, children=[$($wname),*]))]
        #[derive(Clone, Debug)]
        struct L<$($gt: Widget + 'static),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        $crate::impl_layout!(L<$($gt: Widget),*>; 
            $direction; $( $( [ $($pos),* ] )* $wname),*);

        impl<$($gtr, $gt: Widget + Handler<Response = $gtr>),*> Handler
            for L<$($gt),*>
            where $($gtr: From<NoResponse>, $response: From<$gtr>),*
        {
            type Response = $response;
            
            fn handle_action(&mut self, tk: &Toolkit, action: Action, num: u32) -> $response {
                $(
                    if num <= self.$wname.number() {
                        return self.$wname.handle_action(tk, action, num).into();
                    }
                )*
                if num == self.number() {
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
    use crate::widget::{Widget, Layout, Core};
    use crate::display::Text;
    use crate::event::NoResponse;

    #[test]
    #[allow(unused)] // spurious warning in unit test
    fn macro_test_layout() {
        fn check_props<T: Widget + Layout + Core>(_x: T) {}
        
        let w = make_widget!(single<>; self, tk, msg;
            text: Text = Text::from("text") => msg;;
            NoResponse);
        check_props(w);
    }
}
