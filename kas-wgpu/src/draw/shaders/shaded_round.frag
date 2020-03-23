// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) flat in vec3 fragColor;
layout(location = 1) noperspective in vec2 dir;
layout(location = 2) flat in vec2 adjust;
layout(location = 3) noperspective in vec2 off;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 1) uniform Locals {
    vec3 lightNorm;
};

float sample_a(vec2 dir) {
    vec2 dir2 = dir * dir;
    float ss = dir2.x + dir2.y;
    return (ss <= 1.0) ? 0.25 : 0.0;
}

void main() {
    // Multi-sample alpha to avoid ugly aliasing. A single colour sample is adequate.
    vec2 off1 = vec2(off.x, 3.0 * off.y);
    vec2 off2 = vec2(3.0 * off.x, off.y);
    float alpha = sample_a(dir + off1)
        + sample_a(dir - off1)
        + sample_a(dir + off2)
        + sample_a(dir - off2);
    if (alpha == 0.0) discard;

    vec2 dir2 = dir * dir;
    float ss = dir2.x + dir2.y;

    // With multi-sampling we can hit ss>1. Clamp to avoid imaginary roots:
    float z = sqrt(max(1.0 - ss, 0));
    float h = sqrt(ss);
    float t = adjust.x + adjust.y * atan(h, z);
    vec2 normh = vec2(0.0);
    if (h > 0.0) {
        normh = dir * (sin(t) / h);
        z = cos(t);
    }
    vec3 norm = vec3(normh, z);

    // Simplified version with only scale adjustment; looks okay with convex
    // curvature but not with concave:
    // float z = sqrt(1.0 - adjust.y * ss);
    // vec3 norm = vec3(dir * sqrt(adjust.y), z);

    vec3 c = fragColor * dot(norm, lightNorm);
    outColor = vec4(c, alpha);
}
