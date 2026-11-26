// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Block widget

use kas::draw::color::Rgba;
use kas::layout::Scaling;
use kas::prelude::*;
use kas::theme::{MarginStyle, MarkStyle};

#[impl_self]
mod Block {
    /// A basic block
    ///
    /// This widget is not interactive. The block is not painted by default;
    /// several (exclusive) options are supported.
    ///
    /// This is typically used to add an expansion point inside a row or column:
    /// ```
    /// # use kas::{Widget, draw::color::Rgba};
    /// # use kas_widgets::{column, Block};
    /// # fn widget() -> impl Widget<Data = ()> {
    ///     Block::square(100.0).with_rounded_background(10.0, Rgba::WHITE)
    /// # }
    /// ```
    #[autoimpl(Debug ignore self.draw)]
    #[widget]
    pub struct Block {
        core: widget_core!(),
        scaling: Scaling,
        draw: Box<dyn Fn(DrawCx, Rect)>,
    }

    impl Layout for Block {
        fn size_rules(&mut self, cx: &SizeCx, axis: AxisInfo) -> SizeRules {
            self.scaling.size_rules(cx, axis)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            let align = hints.complete_default();
            let rect = self.scaling.align(rect, align, cx.scale_factor());
            self.core.set_rect(rect);
        }

        fn draw(&self, cx: DrawCx) {
            (self.draw)(cx, self.rect());
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::None
        }
    }
}

impl From<Scaling> for Block {
    #[inline]
    fn from(scaling: Scaling) -> Self {
        Block {
            core: Default::default(),
            scaling,
            draw: Box::new(|_, _| ()),
        }
    }
}

impl Block {
    /// Construct a rectangular
    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Block::from(Scaling {
            size: (width, height).into(),
            ..Scaling::default()
        })
    }

    /// Construct a square block
    #[inline]
    pub fn square(size: f32) -> Self {
        Block::new(size, size)
    }

    /// Set a margin style
    ///
    /// By default, this is [`MarginStyle::Large`].
    #[inline]
    pub fn with_margin_style(mut self, style: MarginStyle) -> Self {
        self.scaling.margins = style;
        self
    }

    /// Set stretchiness
    ///
    /// By default, this is [`Stretch::None`].
    #[inline]
    pub fn with_stretch(mut self, stretch: Stretch, fix_aspect: bool) -> Self {
        self.scaling.stretch = stretch;
        self.scaling.fix_aspect = fix_aspect;
        self
    }

    /// Add a draw function (generic)
    #[inline]
    pub fn with_draw_fn(mut self, f: impl Fn(DrawCx, Rect) + 'static) -> Self {
        self.draw = Box::new(f);
        self
    }

    /// Draw a mark
    ///
    /// The mark is not stretched; normally it is recommended to use the
    /// [`Mark`](super::Mark) widget instead.
    #[inline]
    pub fn with_mark(self, style: MarkStyle) -> Self {
        self.with_draw_fn(move |mut cx, rect| cx.mark(rect, style))
    }

    /// Fill self with a background color
    ///
    /// Note: this does not merge with other draw operations. Use
    /// [`Self::with_draw_fn`] instead where required.
    #[inline]
    pub fn with_background(self, color: Rgba) -> Self {
        self.with_draw_fn(move |mut cx, rect| cx.draw().rect(rect.cast(), color))
    }

    /// Fill self with a background color, with rounded corners
    ///
    /// (The rounded corners are backend-dependent: not all drawing backends
    /// support them.)
    ///
    /// Note: this does not merge with other draw operations. Use
    /// [`Self::with_draw_fn`] instead where required.
    #[inline]
    pub fn with_rounded_background(self, radius: f32, color: Rgba) -> Self {
        self.with_draw_fn(move |mut cx, rect| {
            let quad = rect.cast();
            if let Some(draw) = cx.draw_rounded() {
                draw.rounded_rect(quad, radius, color);
            } else {
                cx.draw().rect(quad, color)
            }
        })
    }
}
