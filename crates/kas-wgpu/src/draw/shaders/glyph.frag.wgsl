// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<private> tex_coord_1: vec2<f32>;
var<private> col_1: vec4<f32>;
var<private> outColor: vec4<f32>;
@group(1) @binding(0) 
var tex: texture_2d<f32>;
@group(1) @binding(1) 
var tex_sampler: sampler;

fn main_1() {
    var alpha: f32;

    _ = tex_coord_1;
    let _e6 = tex_coord_1;
    let _e7 = textureSample(tex, tex_sampler, _e6);
    alpha = _e7.x;
    let _e10 = col_1;
    let _e11 = _e10.xyz;
    let _e12 = col_1;
    let _e14 = alpha;
    outColor = vec4<f32>(_e11.x, _e11.y, _e11.z, (_e12.w * _e14));
    return;
}

@fragment 
fn main(@location(0) tex_coord: vec2<f32>, @location(1) col: vec4<f32>) -> FragmentOutput {
    tex_coord_1 = tex_coord;
    col_1 = col;
    main_1();
    let _e15 = outColor;
    return FragmentOutput(_e15);
}
