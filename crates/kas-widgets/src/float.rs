// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A pile of widgets "floating" over the top of each other

use kas::prelude::*;
use kas::Collection;

/// Make a [`Float`] widget
///
/// # Syntax
///
/// > _Collection_ :\
/// > &nbsp;&nbsp; `float!` `[` _Items_<sup>\?</sup> `]`
/// >
/// > _Items_ :\
/// > &nbsp;&nbsp; (_Item_ `,`)<sup>\*</sup> _Item_ `,`<sup>\?</sup>
///
/// ## Stand-alone usage
///
/// When used as a stand-alone macro, `float! [/* ... */]` is just syntactic sugar
/// for `Float::new(kas::collection! [/* ... */])`.
///
/// In this case, _Item_ may be:
///
/// -   A string literal (interpreted as a label widget), optionally followed by
///     an [`align`] or [`pack`] method call
/// -   An expression yielding an object implementing `Widget<Data = _A>`
///
/// In case all _Item_ instances are a string literal, the data type of the
/// `float!` widget will be `()`; otherwise the data type of the widget is `_A`
/// where `_A` is a generic type parameter of the widget.
///
/// ## Usage within widget layout syntax
///
/// When called within [widget layout syntax], `float!` may be evaluated as a
/// recursive macro and the result does not have a specified type, except that
/// methods [`map_any`], [`align`] and [`pack`] are supported via emulation.
///
/// In this case, _Item_ is evaluated using [widget layout syntax]. This is
/// broadly similar to the above with a couple of exceptions:
///
/// -   Supported layout macros do not need to be imported to the module scope
/// -   An _Item_ may be a `#[widget]` field of the widget
///
/// # Example
///
/// ```
/// # use kas::prelude::*;
/// let my_widget = kas_widgets::float! [
///     "one".pack(AlignHints::TOP_LEFT),
///     "two".pack(AlignHints::BOTTOM_RIGHT),
///     "some text\nin the\nbackground"
/// ];
/// ```
///
/// [widget layout syntax]: macro@kas::layout
/// [`map_any`]: crate::AdaptWidgetAny::map_any
/// [`align`]: crate::AdaptWidget::align
/// [`pack`]: crate::AdaptWidget::pack
#[macro_export]
macro_rules! float {
    ( $( $ee:expr ),* ) => {
        $crate::Float::new( ::kas::collection! [ $( $ee ),* ] )
    };
    ( $( $ee:expr ),+ , ) => {
        $crate::Float::new( ::kas::collection! [ $( $ee ),+ ] )
    };
}

#[impl_self]
mod Float {
    /// A float widget
    ///
    /// All widgets occupy the same space with the first child on top.
    ///
    /// Size is determined as the maximum required by any child for each axis.
    /// All widgets are assigned this size. It is usually necessary to use
    /// [`pack`] or a similar mechanism to constrain a child to avoid it hiding
    /// the content underneath (note that even if an unconstrained child does
    /// not *visually* hide everything beneath, it may still "occupy" the
    /// assigned area, preventing mouse clicks from reaching the widget
    /// beneath).
    ///
    /// [`pack`]: crate::AdaptWidget::pack
    #[derive(Clone, Default)]
    #[widget]
    pub struct Float<C: Collection> {
        core: widget_core!(),
        widgets: C,
    }

    impl Self {
        /// Construct a float
        #[inline]
        pub fn new(widgets: C) -> Self {
            Float {
                core: Default::default(),
                widgets,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = SizeRules::EMPTY;
            for i in 0..self.widgets.len() {
                if let Some(child) = self.widgets.get_mut_tile(i) {
                    rules = rules.max(child.size_rules(sizer.re(), axis));
                }
            }
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            for i in 0..self.widgets.len() {
                if let Some(child) = self.widgets.get_mut_tile(i) {
                    child.set_rect(cx, rect, hints);
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let mut iter = (0..self.widgets.len()).rev();
            if let Some(first) = iter.next() {
                if let Some(child) = self.widgets.get_tile(first) {
                    child.draw(draw.re());
                }
            }
            for i in iter {
                if let Some(child) = self.widgets.get_tile(i) {
                    draw.with_pass(|draw| child.draw(draw));
                }
            }
        }
    }

    impl Tile for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }

        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.widgets.get_tile(index)
        }

        fn probe(&self, coord: Coord) -> Id {
            for i in 0..self.widgets.len() {
                if let Some(child) = self.widgets.get_tile(i) {
                    if let Some(id) = child.try_probe(coord) {
                        return id;
                    }
                }
            }
            self.id()
        }
    }

    impl Widget for Self {
        type Data = C::Data;

        #[inline]
        fn child_node<'n>(&'n mut self, data: &'n Self::Data, index: usize) -> Option<Node<'n>> {
            self.widgets.child_node(data, index)
        }
    }
}
