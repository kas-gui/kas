// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) in vec2 tex_coord;
layout(location = 1) in vec4 col;

layout(location = 0, index = 0) out vec4 frag_color;
layout(location = 0, index = 1) out vec4 blend_color;

layout(set = 1, binding = 0) uniform texture2D tex;
layout(set = 1, binding = 1) uniform sampler tex_sampler;

const float gamma = 1.43;

void main() {
    // Get a coverage value of the rastered glyph and use gamma correction to
    // ensure perceptually-linear blending of foreground and background.
    // This assumes a pre-multiplied alpha blend mode.
    vec4 cov = texture(sampler2D(tex, tex_sampler), tex_coord);
    float r = pow(cov.r, gamma);
    float g = pow(cov.g, gamma);
    float b = pow(cov.b, gamma);
    frag_color = vec4(col.rgb * vec3(r, g, b), 0);

    float vr = 1.0 - pow(1.0 - cov.r, gamma);
    float vg = 1.0 - pow(1.0 - cov.g, gamma);
    float vb = 1.0 - pow(1.0 - cov.b, gamma);
    blend_color = col.a * vec4(vr, vg, vb, 0);
}
