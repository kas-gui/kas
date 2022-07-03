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
var<private> dir_3: vec2<f32>;
var<private> adjust_1: vec2<f32>;
var<private> off_1: vec2<f32>;
var<private> outColor: vec4<f32>;
@group(0) @binding(1) 
var<uniform> global: FragCommon;

fn sample_a(dir_1: vec2<f32>) -> f32 {
    var dir_2: vec2<f32>;
    var dir2_: vec2<f32>;
    var ss: f32;
    var local: f32;

    _ = (&global.lightNorm);
    dir_2 = dir_1;
    let _e9 = dir_2;
    let _e10 = dir_2;
    dir2_ = (_e9 * _e10);
    let _e13 = dir2_;
    let _e15 = dir2_;
    ss = (_e13.x + _e15.y);
    let _e19 = ss;
    if (_e19 <= 1.0) {
        local = 0.25;
    } else {
        local = 0.0;
    }
    let _e25 = local;
    return _e25;
}

fn main_1() {
    var off1_: vec2<f32>;
    var off2_: vec2<f32>;
    var alpha: f32;
    var dir2_1: vec2<f32>;
    var ss_1: f32;
    var z: f32;
    var h: f32;
    var t: f32;
    var normh: vec2<f32> = vec2<f32>(0.0, 0.0);
    var norm: vec3<f32>;
    var c: vec3<f32>;

    let _e7 = off_1;
    off1_ = vec2<f32>(_e7.x, 0.0);
    let _e13 = off_1;
    off2_ = vec2<f32>(0.0, _e13.y);
    let _e17 = dir_3;
    let _e18 = off1_;
    _ = (_e17 + _e18);
    let _e20 = dir_3;
    let _e21 = off1_;
    let _e23 = sample_a((_e20 + _e21));
    let _e24 = dir_3;
    let _e25 = off1_;
    _ = (_e24 - _e25);
    let _e27 = dir_3;
    let _e28 = off1_;
    let _e30 = sample_a((_e27 - _e28));
    let _e32 = dir_3;
    let _e33 = off2_;
    _ = (_e32 + _e33);
    let _e35 = dir_3;
    let _e36 = off2_;
    let _e38 = sample_a((_e35 + _e36));
    let _e40 = dir_3;
    let _e41 = off2_;
    _ = (_e40 - _e41);
    let _e43 = dir_3;
    let _e44 = off2_;
    let _e46 = sample_a((_e43 - _e44));
    alpha = (((_e23 + _e30) + _e38) + _e46);
    let _e49 = alpha;
    if (_e49 == 0.0) {
        discard;
    }
    let _e52 = dir_3;
    let _e53 = dir_3;
    dir2_1 = (_e52 * _e53);
    let _e56 = dir2_1;
    let _e58 = dir2_1;
    ss_1 = (_e56.x + _e58.y);
    let _e63 = ss_1;
    _ = (1.0 - _e63);
    let _e67 = ss_1;
    _ = max((1.0 - _e67), f32(0));
    let _e73 = ss_1;
    _ = (1.0 - _e73);
    let _e77 = ss_1;
    z = sqrt(max((1.0 - _e77), f32(0)));
    _ = ss_1;
    let _e85 = ss_1;
    h = sqrt(_e85);
    let _e88 = adjust_1;
    let _e90 = adjust_1;
    _ = h;
    _ = z;
    let _e94 = h;
    let _e95 = z;
    t = (_e88.x + (_e90.y * atan2(_e94, _e95)));
    _ = vec2<f32>(0.0);
    let _e103 = h;
    if (_e103 > 0.0) {
        {
            let _e106 = dir_3;
            _ = t;
            let _e108 = t;
            let _e110 = h;
            normh = (_e106 * (sin(_e108) / _e110));
            _ = t;
            let _e114 = t;
            z = cos(_e114);
        }
    }
    let _e116 = normh;
    let _e117 = z;
    norm = vec3<f32>(_e116.x, _e116.y, _e117);
    let _e122 = fragColor_1;
    _ = norm;
    _ = global.lightNorm;
    let _e126 = norm;
    let _e127 = global.lightNorm;
    c = (_e122.xyz * dot(_e126, _e127));
    let _e131 = c;
    let _e132 = fragColor_1;
    let _e134 = alpha;
    outColor = vec4<f32>(_e131.x, _e131.y, _e131.z, (_e132.w * _e134));
    return;
}

@fragment 
fn main(@location(0) fragColor: vec4<f32>, @location(1) dir: vec2<f32>, @location(2) adjust: vec2<f32>, @location(3) off: vec2<f32>) -> FragmentOutput {
    fragColor_1 = fragColor;
    dir_3 = dir;
    adjust_1 = adjust;
    off_1 = off;
    main_1();
    let _e21 = outColor;
    return FragmentOutput(_e21);
}
