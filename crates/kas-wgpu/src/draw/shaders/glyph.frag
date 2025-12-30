// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

#version 450
#extension GL_ARB_separate_shader_objects : enable

precision mediump float;

layout(location = 0) in vec2 tex_coord;
layout(location = 1) in vec4 col;

layout(location = 0) out vec4 outColor;

layout(set = 1, binding = 0) uniform texture2D tex;
layout(set = 1, binding = 1) uniform sampler tex_sampler;

const float gamma = 1.43;

void main() {
    // Get a coverage value of the rastered glyph and use gamma correction to
    // ensure perceptually-linear blending of foreground and background.
    // This assumes a pre-multiplied alpha blend mode.
    float cov = texture(sampler2D(tex, tex_sampler), tex_coord).r;
    vec3 rgb = col.rgb * pow(cov, gamma);
    float inv_cov = 1.0 - pow(1.0 - cov, gamma);
    outColor = vec4(rgb, col.a * inv_cov);
}
