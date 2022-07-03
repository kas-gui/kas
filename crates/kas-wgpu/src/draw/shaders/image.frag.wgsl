// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<private> tex_coord_1: vec2<f32>;
var<private> outColor: vec4<f32>;
@group(1) @binding(0) 
var tex: texture_2d<f32>;
@group(1) @binding(1) 
var tex_sampler: sampler;

fn main_1() {
    _ = tex_coord_1;
    let _e5 = tex_coord_1;
    let _e6 = textureSample(tex, tex_sampler, _e5);
    outColor = _e6;
    return;
}

@fragment 
fn main(@location(0) tex_coord: vec2<f32>) -> FragmentOutput {
    tex_coord_1 = tex_coord;
    main_1();
    let _e11 = outColor;
    return FragmentOutput(_e11);
}
