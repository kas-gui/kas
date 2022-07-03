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
    @builtin(position) member: vec4<f32>,
}

var<private> a_pos_1: vec2<f32>;
var<private> a_col_1: vec4<f32>;
var<private> a1_1: vec2<f32>;
var<private> b_col: vec4<f32>;
var<private> b1_: vec2<f32>;
@group(0) @binding(0) 
var<uniform> global: VertexCommon;
var<private> gl_Position: vec4<f32>;

fn main_1() {
    let _e10 = global.scale;
    let _e11 = a_pos_1;
    let _e13 = global.offset;
    let _e15 = (_e10 * (_e11.xy + _e13));
    gl_Position = vec4<f32>(_e15.x, _e15.y, 0.0, 1.0);
    let _e21 = a_col_1;
    b_col = _e21;
    let _e22 = a1_1;
    b1_ = _e22;
    return;
}

@vertex 
fn main(@location(0) a_pos: vec2<f32>, @location(1) a_col: vec4<f32>, @location(2) a1_: vec2<f32>) -> VertexOutput {
    a_pos_1 = a_pos;
    a_col_1 = a_col;
    a1_1 = a1_;
    _ = (&global.offset);
    main_1();
    let _e21 = b_col;
    let _e23 = b1_;
    let _e25 = gl_Position;
    return VertexOutput(_e21, _e23, _e25);
}
