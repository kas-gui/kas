// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

// Low precision seems to be good enough
precision lowp float;

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec2 dir;
layout(location = 2) in vec2 adjust;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 1) uniform Locals {
    vec3 lightNorm;
};

void main() {
    vec2 dir2 = dir * dir;
    float ss = dir2.x + dir2.y;
    if (ss > 1.0) discard;
    
    float z = sqrt(1.0 - ss);
    float h = sqrt(ss);
    float t = adjust.x + adjust.y * atan(h / z);
    vec2 normh;
    if (h > 0.0) {
        normh = dir * (sin(t) / h);
        z = cos(t);
    }
    vec3 norm = vec3(normh, z);
    
    // Simplified version with only scale adjustment:
    // float z = sqrt(1.0 - adjust.y * ss);
    // vec3 norm = vec3(dir * sqrt(adjust.y), z);
    
    vec3 c = fragColor * max(dot(norm, lightNorm), 0);
    outColor = vec4(c, 1.0);
}
