// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct VertexCommon {
    offset: vec2<f32>,
    scale: vec2<f32>,
}

struct VertexOutput {
    @location(0) b1_: vec2<f32>,
    @builtin(position) member: vec4<f32>,
}

var<private> a_pos_1: vec3<f32>;
var<private> a1_1: vec2<f32>;
var<private> b1_: vec2<f32>;
@group(0) @binding(0) 
var<uniform> global: VertexCommon;
var<private> gl_Position: vec4<f32>;

fn main_1() {
    let _e8 = global.scale;
    let _e9 = a_pos_1;
    let _e11 = global.offset;
    let _e13 = (_e8 * (_e9.xy + _e11));
    gl_Position = vec4<f32>(_e13.x, _e13.y, 0.0, 1.0);
    let _e19 = a1_1;
    b1_ = _e19;
    return;
}

@vertex 
fn main(@location(0) a_pos: vec3<f32>, @location(1) a1_: vec2<f32>) -> VertexOutput {
    a_pos_1 = a_pos;
    a1_1 = a1_;
    _ = (&global.offset);
    main_1();
    let _e15 = b1_;
    let _e17 = gl_Position;
    return VertexOutput(_e15, _e17);
}
