// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec2 dir;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 1) uniform Locals {
    vec3 lightNorm;
};


void main() {
    vec2 dir2 = abs(dir) * dir;
    float ss = abs(dir2.x) + abs(dir2.y);
    if (ss < 0.0 || ss > 1.0) discard;
    
    // TODO: allow scaling and inversion?
    vec2 norm2 = dir2;
    
    float n3 = 1.0 - sqrt(norm2.x * norm2.x + norm2.y * norm2.y);
    vec3 norm = vec3(norm2, n3);
    
    vec3 c = fragColor * max(dot(norm, lightNorm), 0);
    outColor = vec4(c, 1.0);
}
