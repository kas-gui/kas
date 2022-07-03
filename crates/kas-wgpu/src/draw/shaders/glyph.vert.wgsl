// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct VertexCommon {
    offset: vec2<f32>,
    scale: vec2<f32>,
}

struct VertexOutput {
    @location(0) tex_pos: vec2<f32>,
    @location(1) outColor: vec4<f32>,
    @builtin(position) member: vec4<f32>,
}

var<private> pos_a_1: vec2<f32>;
var<private> pos_b_1: vec2<f32>;
var<private> tex_a_1: vec2<f32>;
var<private> tex_b_1: vec2<f32>;
var<private> inColor_1: vec4<f32>;
var<private> tex_pos: vec2<f32>;
var<private> outColor: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: VertexCommon;
var<private> gl_VertexIndex: u32;
var<private> gl_Position: vec4<f32>;

fn main_1() {
    var pos: vec2<f32>;

    let _e13 = gl_VertexIndex;
    switch _e13 {
        case 0u: {
            let _e15 = pos_a_1;
            pos = _e15;
            let _e16 = tex_a_1;
            tex_pos = _e16;
        }
        case 1u: {
            let _e18 = pos_b_1;
            let _e20 = pos_a_1;
            pos = vec2<f32>(_e18.x, _e20.y);
            let _e23 = tex_b_1;
            let _e25 = tex_a_1;
            tex_pos = vec2<f32>(_e23.x, _e25.y);
        }
        case 2u: {
            let _e29 = pos_a_1;
            let _e31 = pos_b_1;
            pos = vec2<f32>(_e29.x, _e31.y);
            let _e34 = tex_a_1;
            let _e36 = tex_b_1;
            tex_pos = vec2<f32>(_e34.x, _e36.y);
        }
        case 3u: {
            let _e40 = pos_b_1;
            pos = _e40;
            let _e41 = tex_b_1;
            tex_pos = _e41;
        }
        default: {
        }
    }
    let _e42 = inColor_1;
    outColor = _e42;
    let _e44 = global.scale;
    let _e45 = pos;
    let _e47 = global.offset;
    let _e49 = (_e44 * (_e45.xy + _e47));
    gl_Position = vec4<f32>(_e49.x, _e49.y, 0.0, 1.0);
    return;
}

@vertex 
fn main(@location(0) pos_a: vec2<f32>, @location(1) pos_b: vec2<f32>, @location(2) tex_a: vec2<f32>, @location(3) tex_b: vec2<f32>, @location(4) inColor: vec4<f32>, @builtin(vertex_index) param: u32) -> VertexOutput {
    pos_a_1 = pos_a;
    pos_b_1 = pos_b;
    tex_a_1 = tex_a;
    tex_b_1 = tex_b;
    inColor_1 = inColor;
    gl_VertexIndex = param;
    _ = (&global.offset);
    main_1();
    let _e31 = tex_pos;
    let _e33 = outColor;
    let _e35 = gl_Position;
    return VertexOutput(_e31, _e33, _e35);
}
