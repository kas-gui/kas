// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 a_Pos;
layout(location = 1) in vec3 a_Color;

layout(location = 0) out vec3 fragColor;

layout(set = 0, binding = 0) uniform Locals {
    vec2 scale;
};

const vec2 offset = { 1.0, 1.0 };

void main() {
    gl_Position = vec4(scale * a_Pos - offset, 0.0, 1.0);
    fragColor = a_Color;
}
