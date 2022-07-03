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
    @builtin(position) member: vec4<f32>,
}

var<private> pos_a_1: vec2<f32>;
var<private> pos_b_1: vec2<f32>;
var<private> tex_a_1: vec2<f32>;
var<private> tex_b_1: vec2<f32>;
var<private> tex_pos: vec2<f32>;
@group(0) @binding(0) 
var<uniform> global: VertexCommon;
var<private> gl_VertexIndex: u32;
var<private> gl_Position: vec4<f32>;

fn main_1() {
    var pos: vec2<f32>;

    let _e11 = gl_VertexIndex;
    switch _e11 {
        case 0u: {
            let _e13 = pos_a_1;
            pos = _e13;
            let _e14 = tex_a_1;
            tex_pos = _e14;
        }
        case 1u: {
            let _e16 = pos_b_1;
            let _e18 = pos_a_1;
            pos = vec2<f32>(_e16.x, _e18.y);
            let _e21 = tex_b_1;
            let _e23 = tex_a_1;
            tex_pos = vec2<f32>(_e21.x, _e23.y);
        }
        case 2u: {
            let _e27 = pos_a_1;
            let _e29 = pos_b_1;
            pos = vec2<f32>(_e27.x, _e29.y);
            let _e32 = tex_a_1;
            let _e34 = tex_b_1;
            tex_pos = vec2<f32>(_e32.x, _e34.y);
        }
        case 3u: {
            let _e38 = pos_b_1;
            pos = _e38;
            let _e39 = tex_b_1;
            tex_pos = _e39;
        }
        default: {
        }
    }
    let _e41 = global.scale;
    let _e42 = pos;
    let _e44 = global.offset;
    let _e46 = (_e41 * (_e42.xy + _e44));
    gl_Position = vec4<f32>(_e46.x, _e46.y, 0.0, 1.0);
    return;
}

@vertex 
fn main(@location(0) pos_a: vec2<f32>, @location(1) pos_b: vec2<f32>, @location(2) tex_a: vec2<f32>, @location(3) tex_b: vec2<f32>, @builtin(vertex_index) param: u32) -> VertexOutput {
    pos_a_1 = pos_a;
    pos_b_1 = pos_b;
    tex_a_1 = tex_a;
    tex_b_1 = tex_b;
    gl_VertexIndex = param;
    _ = (&global.offset);
    main_1();
    let _e25 = tex_pos;
    let _e27 = gl_Position;
    return VertexOutput(_e25, _e27);
}
