// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct VertexCommon {
    offset: vec2<f32>,
    scale: vec2<f32>,
}

struct VertexOutput {
    @location(0) @interpolate(flat) b_col1_: vec4<f32>,
    @location(1) @interpolate(flat) b_col2_: vec4<f32>,
    @location(2) b_v: vec2<f32>,
    @builtin(position) member: vec4<f32>,
}

var<private> a_pos_1: vec2<f32>;
var<private> a_col1_1: vec4<f32>;
var<private> a_col2_1: vec4<f32>;
var<private> a_v_1: vec2<f32>;
var<private> b_col1_: vec4<f32>;
var<private> b_col2_: vec4<f32>;
var<private> b_v: vec2<f32>;
@group(0) @binding(0) 
var<uniform> global: VertexCommon;
var<private> gl_Position: vec4<f32>;

fn main_1() {
    let _e12 = global.scale;
    let _e13 = a_pos_1;
    let _e15 = global.offset;
    let _e17 = (_e12 * (_e13.xy + _e15));
    gl_Position = vec4<f32>(_e17.x, _e17.y, 0.0, 1.0);
    let _e23 = a_col1_1;
    b_col1_ = _e23;
    let _e24 = a_col2_1;
    b_col2_ = _e24;
    let _e25 = a_v_1;
    b_v = _e25;
    return;
}

@vertex 
fn main(@location(0) a_pos: vec2<f32>, @location(1) a_col1_: vec4<f32>, @location(2) a_col2_: vec4<f32>, @location(3) a_v: vec2<f32>) -> VertexOutput {
    a_pos_1 = a_pos;
    a_col1_1 = a_col1_;
    a_col2_1 = a_col2_;
    a_v_1 = a_v;
    _ = (&global.offset);
    main_1();
    let _e27 = b_col1_;
    let _e29 = b_col2_;
    let _e31 = b_v;
    let _e33 = gl_Position;
    return VertexOutput(_e27, _e29, _e31, _e33);
}
