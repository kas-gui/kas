// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) flat in vec3 fragColor;
layout(location = 1) flat in float inner;
layout(location = 2) noperspective in vec2 pos;
layout(location = 3) noperspective in vec2 off;

layout(location = 0) out vec4 outColor;

float sample_a(vec2 pos) {
    vec2 pos2 = pos * pos;
    float ss = pos2.x + pos2.y;
    return (inner <= ss && ss <= 1.0) ? 0.25 : 0.0;
}

void main() {
    // Multi-sample alpha to avoid ugly aliasing.
    vec2 off1 = vec2(off.x, 3.0 * off.y);
    vec2 off2 = vec2(3.0 * off.x, off.y);
    float alpha = sample_a(pos + off1)
        + sample_a(pos - off1)
        + sample_a(pos + off2)
        + sample_a(pos - off2);

    outColor = vec4(fragColor, alpha);
}
