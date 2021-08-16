// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) flat in vec4 col1;
layout(location = 1) flat in vec4 col2;
layout(location = 2) in vec2 pos;

layout(location = 0) out vec4 outColor;

void main() {
    vec2 pos2 = pos * pos;
    float ss = pos2.x + pos2.y;
    if (!(ss <= 1.0)) {
        discard;
    }
    float r = sqrt(ss);
    outColor = mix(col1, col2, r);
}
