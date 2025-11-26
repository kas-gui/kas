// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Parley text

use cast::Cast;
use kas_text::Align;
use parley::{Alignment, AlignmentOptions};
use crate::event::ConfigCx;
use crate::geom::Rect;
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::theme::{DrawCx, SizeCx, TextBrush, TextClass};

#[derive(Clone)]
pub struct ParleyText {
    class: TextClass,
    rect: Rect,
    layout: parley::Layout<TextBrush>,
}

impl crate::Layout for ParleyText {
    #[inline]
    fn rect(&self) -> Rect {
        self.rect
    }

    #[inline]
    fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        // Measure size and perform line-breaking (vertical axis only):
        sizer.parley_rules(&mut self.layout, self.class, axis)
    }

    fn set_rect(&mut self, _: &mut ConfigCx, rect: Rect, hints: AlignHints) {
        self.rect = rect;

        // TODO: revise alignment; allow local override?
        let alignment = match hints.horiz {
            None => Alignment::Start,
            Some(Align::Default) => Alignment::Start,
            Some(Align::TL) => Alignment::Left,
            Some(Align::Center) => Alignment::Middle,
            Some(Align::BR) => Alignment::Right,
            Some(Align::Stretch) => Alignment::Justified,
        };
        let options = AlignmentOptions { align_when_overflowing: true };
        self.layout.align(Some(rect.size.0.cast()), alignment, options);
    }

    fn draw(&self, mut draw: DrawCx) {
        draw.parley(self.rect, &self.layout);
    }
}
