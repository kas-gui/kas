// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::images::{Images as Pipeline, InstanceA, InstanceRgba, Window};
use kas::draw::{PassId, color::Rgba};
use kas::geom::{Quad, Vec2};
use kas::text::raster::{RenderQueue, Sprite};
use kas_text::{Effect, TextDisplay};

impl RenderQueue for Window {
    fn push_sprite(
        &mut self,
        pass: PassId,
        glyph_pos: Vec2,
        rect: Quad,
        col: Rgba,
        sprite: &Sprite,
    ) {
        let mut a = glyph_pos.floor() + sprite.offset;
        let mut b = a + sprite.size;

        if !(sprite.is_valid()
            && a.0 < rect.b.0
            && a.1 < rect.b.1
            && b.0 > rect.a.0
            && b.1 > rect.a.1)
        {
            return;
        }

        let (mut ta, mut tb) = (sprite.tex_quad.a, sprite.tex_quad.b);
        if !(a >= rect.a) || !(b <= rect.b) {
            let size_inv = Vec2::splat(1.0) / (b - a);
            let fa0 = 0f32.max((rect.a.0 - a.0) * size_inv.0);
            let fa1 = 0f32.max((rect.a.1 - a.1) * size_inv.1);
            let fb0 = 1f32.min((rect.b.0 - a.0) * size_inv.0);
            let fb1 = 1f32.min((rect.b.1 - a.1) * size_inv.1);

            let ts = tb - ta;
            tb = ta + ts * Vec2(fb0, fb1);
            ta += ts * Vec2(fa0, fa1);

            a.0 = a.0.clamp(rect.a.0, rect.b.0);
            a.1 = a.1.clamp(rect.a.1, rect.b.1);
            b.0 = b.0.clamp(rect.a.0, rect.b.0);
            b.1 = b.1.clamp(rect.a.1, rect.b.1);
        }

        if !sprite.color {
            let instance = InstanceA { a, b, ta, tb, col };
            self.atlas_a.rect(pass, sprite.atlas, instance);
        } else {
            let instance = InstanceRgba { a, b, ta, tb };
            self.atlas_rgba.rect(pass, sprite.atlas, instance);
        }
    }
}

impl Window {
    pub fn text(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        pos: Vec2,
        bb: Quad,
        text: &TextDisplay,
        col: Rgba,
    ) {
        pipe.text.text(
            self,
            &mut pipe.atlas_a,
            &mut pipe.atlas_rgba,
            pass,
            pos,
            bb,
            text,
            col,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text_effects(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        pos: Vec2,
        bb: Quad,
        text: &TextDisplay,
        effects: &[Effect],
        colors: &[Rgba],
        draw_quad: impl FnMut(Quad, Rgba),
    ) {
        pipe.text.text_effects(
            self,
            &mut pipe.atlas_a,
            &mut pipe.atlas_rgba,
            pass,
            pos,
            bb,
            text,
            effects,
            colors,
            draw_quad,
        );
    }
}
