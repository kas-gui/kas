// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct VertexCommon {
    offset: vec2<f32>,
    scale: vec2<f32>,
}
@group(0) @binding(0)
var<uniform> global: VertexCommon;

struct FragCommon {
    lightNorm: vec3<f32>,
    _padding: f32,
}
@group(0) @binding(1)
var<uniform> global2: FragCommon;

struct VertexOutput {
    @location(0) fragColor: vec4<f32>,
    @location(1) norm2: vec2<f32>,
    @builtin(position) member: vec4<f32>,
}

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

@vertex
fn vert(
    @location(0) a_pos: vec2<f32>,
    @location(1) a_col: vec4<f32>,
    @location(2) a1: vec2<f32>,
) -> VertexOutput {
    let pos = global.scale * (a_pos.xy + global.offset);
    let gl_Position = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    return VertexOutput(a_col, a1, gl_Position);
}

@fragment 
fn frag(
    @location(0) fragColor: vec4<f32>,
    @location(1) norm2: vec2<f32>,
) -> FragmentOutput {
    let n3: f32 = sqrt(1.0 - norm2.x * norm2.x - norm2.y * norm2.y);
    let norm = vec3<f32>(norm2.x, norm2.y, n3);
    let c: vec3<f32> = (fragColor.xyz * dot(norm, global2.lightNorm));
    let outColor = vec4<f32>(c.x, c.y, c.z, fragColor.w);
    return FragmentOutput(outColor);
}
