// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct FragCommon {
    lightNorm: vec3<f32>,
}

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<private> fragColor_1: vec4<f32>;
var<private> norm2_1: vec2<f32>;
var<private> outColor: vec4<f32>;
@group(0) @binding(1) 
var<uniform> global: FragCommon;

fn main_1() {
    var n3_: f32;
    var norm: vec3<f32>;
    var c: vec3<f32>;

    let _e6 = norm2_1;
    let _e8 = norm2_1;
    let _e11 = norm2_1;
    let _e13 = norm2_1;
    _ = ((_e6.x * _e8.x) + (_e11.y * _e13.y));
    let _e17 = norm2_1;
    let _e19 = norm2_1;
    let _e22 = norm2_1;
    let _e24 = norm2_1;
    n3_ = (1.0 - sqrt(((_e17.x * _e19.x) + (_e22.y * _e24.y))));
    let _e31 = norm2_1;
    let _e32 = n3_;
    norm = vec3<f32>(_e31.x, _e31.y, _e32);
    let _e37 = fragColor_1;
    _ = norm;
    _ = global.lightNorm;
    let _e41 = norm;
    let _e42 = global.lightNorm;
    c = (_e37.xyz * dot(_e41, _e42));
    let _e46 = c;
    let _e47 = fragColor_1;
    outColor = vec4<f32>(_e46.x, _e46.y, _e46.z, _e47.w);
    return;
}

@fragment 
fn main(@location(0) fragColor: vec4<f32>, @location(1) norm2_: vec2<f32>) -> FragmentOutput {
    fragColor_1 = fragColor;
    norm2_1 = norm2_;
    main_1();
    let _e13 = outColor;
    return FragmentOutput(_e13);
}
