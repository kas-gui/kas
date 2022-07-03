// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct VertexCommon {
    offset: vec2<f32>,
    scale: vec2<f32>,
}

struct VertexOutput {
    @location(0) b_col: vec4<f32>,
    @location(1) b1_: vec2<f32>,
    @location(2) b2_: vec2<f32>,
    @location(3) b3_: vec2<f32>,
    @builtin(position) member: vec4<f32>,
}

var<private> a_pos_1: vec2<f32>;
var<private> a_col_1: vec4<f32>;
var<private> a1_1: vec2<f32>;
var<private> a2_1: vec2<f32>;
var<private> a3_1: vec2<f32>;
var<private> b_col: vec4<f32>;
var<private> b1_: vec2<f32>;
var<private> b2_: vec2<f32>;
var<private> b3_: vec2<f32>;
@group(0) @binding(0) 
var<uniform> global: VertexCommon;
var<private> gl_Position: vec4<f32>;

fn main_1() {
    let _e14 = global.scale;
    let _e15 = a_pos_1;
    let _e17 = global.offset;
    let _e19 = (_e14 * (_e15.xy + _e17));
    gl_Position = vec4<f32>(_e19.x, _e19.y, 0.0, 1.0);
    let _e25 = a_col_1;
    b_col = _e25;
    let _e26 = a1_1;
    b1_ = _e26;
    let _e27 = a2_1;
    b2_ = _e27;
    let _e28 = a3_1;
    b3_ = _e28;
    return;
}

@vertex 
fn main(@location(0) a_pos: vec2<f32>, @location(1) a_col: vec4<f32>, @location(2) a1_: vec2<f32>, @location(3) a2_: vec2<f32>, @location(4) a3_: vec2<f32>) -> VertexOutput {
    a_pos_1 = a_pos;
    a_col_1 = a_col;
    a1_1 = a1_;
    a2_1 = a2_;
    a3_1 = a3_;
    _ = (&global.offset);
    main_1();
    let _e33 = b_col;
    let _e35 = b1_;
    let _e37 = b2_;
    let _e39 = b3_;
    let _e41 = gl_Position;
    return VertexOutput(_e33, _e35, _e37, _e39, _e41);
}
