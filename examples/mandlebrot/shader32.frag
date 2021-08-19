// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision highp float;

layout(location = 0) in vec2 cf;

layout(location = 0) out vec4 outColor;

layout(push_constant) uniform Locals {
    vec2 alpha;
    vec2 delta;
    int iter;
};

void main() {
    vec2 cd = cf;
    vec2 c = vec2(alpha.x * cd.x - alpha.y * cd.y, alpha.x * cd.y + alpha.y * cd.x) + delta;

    vec2 z = c;
    int i;
    for(i=0; i<iter; i++) {
        float x = (z.x * z.x - z.y * z.y) + c.x;
        float y = (z.y * z.x + z.x * z.y) + c.y;

        if((x * x + y * y) > 4.0) break;
        z.x = x;
        z.y = y;
    }

    float r = (i == iter) ? 0.0 : float(i) / iter;
    float g = r * r;
    float b = g * g;
    outColor = vec4(r, g, b, 1.0);
}
