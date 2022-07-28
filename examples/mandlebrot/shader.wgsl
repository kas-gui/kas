struct VertexCommon {
    offset: vec2<f32>,
    scale: vec2<f32>,
}

struct VertexOutput {
    @location(0) b1_: vec2<f32>,
    @builtin(position) member: vec4<f32>,
}

@group(0) @binding(0) 
var<uniform> global: VertexCommon;

@vertex 
fn main(@location(0) a_pos: vec3<f32>, @location(1) a1_: vec2<f32>) -> VertexOutput {
    let pos = (global.scale * (a_pos.xy + global.offset));
    let gl_Position = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    return VertexOutput(a1_, gl_Position);
}
