//! The impl_widget and make_widget macros

#[macro_export]
macro_rules! count_items {
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + count_items!($($rest),*)
    }
}

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
    ($ty:ident, $core:ident) => {
        impl_widget_core!($ty<>, $core);
    };
}

/// Construct a container widget
#[macro_export]
macro_rules! make_layout {
    ($direction:ident < $($gt:ident [$gtr:path]),* >;
        $self:ident, $tk: ident, $msg:ident;
        $($wname:ident: $wt:ident = $wvalue:expr => $whandle:expr),* ;
        $($dname:ident: $dt:ident = $dvalue:expr),* ;
        $response:path) =>
    {{
        use $crate::event::{Action, Handler, ignore};
        use $crate::toolkit::Toolkit;
        use $crate::widget::{Class, CoreData, WidgetCore, Widget, Layout};

        #[derive(Clone, Debug)]
        struct L<$($gt: Widget + 'static),*> {
            core: CoreData,
            $($wname: $wt),* ,
            $($dname: $dt),*
        }

        impl_widget_core!(L<$($gt: Widget),*>, core);

        impl<$($gt: Widget),*> Layout for L<$($gt),*> {
            fn child_layout(&self) -> $crate::widget::ChildLayout {
                select_child_layout!($direction)
            }

            layout_init_constraints!($direction; $($wname),*);
            layout_apply_constraints!($direction; $($wname),*);
            
            fn sync_size(&mut self, tk: &Toolkit) {
                let new_rect = tk.tk_widget().get_rect(self.get_tkd());
                *self.rect_mut() = new_rect;
                
                $(self.$wname.sync_size(tk);)*
            }
        }

        impl<$($gt: Widget + 'static),*> Widget for L<$($gt),*>
        {
            fn class(&self) -> Class { Class::Container }
            fn label(&self) -> Option<&str> { None }

            fn len(&self) -> usize {
                count_items!($($wname),*)
            }
            fn get(&self, index: usize) -> Option<&Widget> {
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
            fn get_mut(&mut self, index: usize) -> Option<&mut Widget> {
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
    }}
}

#[cfg(test)] mod test {
    use crate::widget::{Widget, Layout, WidgetCore, canvas::Text};
    use crate::event::NoResponse;

    #[test]
    fn macro_test_layout() {
        fn check_props<T: Widget + Layout + WidgetCore>(_x: T) {}
        
        let w = make_layout!(single<>; self, tk, msg;
            text: Text = Text::from("text") => msg;;
            NoResponse);
        check_props(w);
    }
}
