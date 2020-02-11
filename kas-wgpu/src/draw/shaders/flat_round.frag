// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

// Low precision seems to be good enough
precision mediump float;

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec2 dir;
layout(location = 2) in vec2 off;

layout(location = 0) out vec4 outColor;

float sample_off(vec2 dir) {
    vec2 dir2 = dir * dir;
    float ss = dir2.x + dir2.y;
    return (ss <= 1.0) ? 0.5 : 0.0;
}

void main() {
    float avg = sample_off(dir + off) + sample_off(dir - off);
    
    outColor = vec4(fragColor, avg);
}
