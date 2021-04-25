// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::{CustomWindow, DrawWindow};
use kas::draw::{Colour, DrawText, Pass};
use kas::geom::Vec2;
use kas::text::{Effect, TextDisplay};

impl<CW: CustomWindow> DrawText for DrawWindow<CW> {
    fn text_col_effects(
        &mut self,
        pass: Pass,
        pos: Vec2,
        bounds: Vec2,
        offset: Vec2,
        text: &TextDisplay,
        col: Colour,
        effects: &[Effect<()>],
    ) {
        // TODO
        let _ = (pass, pos, bounds, offset, text, col, effects);
    }

    fn text_effects(
        &mut self,
        pass: Pass,
        pos: Vec2,
        bounds: Vec2,
        offset: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Colour>],
    ) {
        // TODO
        let _ = (pass, pos, bounds, offset, text, effects);
    }
}
