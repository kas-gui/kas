// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec4 a_col;
layout(location = 2) in float a1;
layout(location = 3) in vec2 a2;
layout(location = 4) in vec2 a3;

layout(location = 0) out vec4 b_col;
layout(location = 1) out float b1;
layout(location = 2) out vec2 b2;
layout(location = 3) out vec2 b3;

layout(set = 0, binding = 0) uniform Locals {
    vec2 scale;
};

const vec2 offset = { -1.0, 1.0 };

void main() {
    gl_Position = vec4(scale * a_pos.xy + offset, 0.0, 1.0);
    b_col = a_col;
    b1 = a1;
    b2 = a2;
    b3 = a3;
}
