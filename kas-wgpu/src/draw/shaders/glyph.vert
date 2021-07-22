// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) in vec2 pos_a;
layout(location = 1) in vec2 pos_b;
layout(location = 2) in vec2 tex_a;
layout(location = 3) in vec2 tex_b;
layout(location = 4) in vec4 inColor;

layout(location = 0) out vec2 tex_pos;
layout(location = 1) out vec4 outColor;

layout(set = 0, binding = 0) uniform VertexCommon {
    vec2 scale;
};

const vec2 offset = { -1.0, 1.0 };

void main() {
    vec2 pos;
    switch (gl_VertexIndex) {
        case 0:
            pos = pos_a;
            tex_pos = tex_a;
            break;
        case 1:
            pos = vec2(pos_b.x, pos_a.y);
            tex_pos = vec2(tex_b.x, tex_a.y);
            break;
        case 2:
            pos = vec2(pos_a.x, pos_b.y);
            tex_pos = vec2(tex_a.x, tex_b.y);
            break;
        case 3:
            pos = pos_b;
            tex_pos = tex_b;
            break;
    }

    outColor = inColor;

    gl_Position = vec4(scale * pos.xy + offset, 0.0, 1.0);
}
