// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec4 a_col1;
layout(location = 2) in vec4 a_col2;
layout(location = 3) in vec2 a_v;

layout(location = 0) flat out vec4 b_col1;
layout(location = 1) flat out vec4 b_col2;
layout(location = 2) out vec2 b_v;

layout(set = 0, binding = 0) uniform VertexCommon {
    vec2 offset;
    vec2 scale;
};

void main() {
    gl_Position = vec4(scale * (a_pos.xy + offset), 0.0, 1.0);
    b_col1 = a_col1;
    b_col2 = a_col2;
    b_v = a_v;
}
