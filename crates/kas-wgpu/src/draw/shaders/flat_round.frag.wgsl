// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<private> fragColor_1: vec4<f32>;
var<private> inner_1: f32;
var<private> pos_3: vec2<f32>;
var<private> off_1: vec2<f32>;
var<private> outColor: vec4<f32>;

fn sample_a(pos_1: vec2<f32>) -> f32 {
    var pos_2: vec2<f32>;
    var pos2_: vec2<f32>;
    var ss: f32;
    var local: f32;

    pos_2 = pos_1;
    let _e7 = pos_2;
    let _e8 = pos_2;
    pos2_ = (_e7 * _e8);
    let _e11 = pos2_;
    let _e13 = pos2_;
    ss = (_e11.x + _e13.y);
    let _e17 = inner_1;
    let _e18 = ss;
    let _e20 = ss;
    if ((_e17 <= _e18) && (_e20 <= 1.0)) {
        local = 0.25;
    } else {
        local = 0.0;
    }
    let _e27 = local;
    return _e27;
}

fn main_1() {
    var off1_: vec2<f32>;
    var off2_: vec2<f32>;
    var alpha: f32;

    let _e5 = off_1;
    off1_ = vec2<f32>(_e5.x, 0.0);
    let _e11 = off_1;
    off2_ = vec2<f32>(0.0, _e11.y);
    let _e15 = pos_3;
    let _e16 = off1_;
    _ = (_e15 + _e16);
    let _e18 = pos_3;
    let _e19 = off1_;
    let _e21 = sample_a((_e18 + _e19));
    let _e22 = pos_3;
    let _e23 = off1_;
    _ = (_e22 - _e23);
    let _e25 = pos_3;
    let _e26 = off1_;
    let _e28 = sample_a((_e25 - _e26));
    let _e30 = pos_3;
    let _e31 = off2_;
    _ = (_e30 + _e31);
    let _e33 = pos_3;
    let _e34 = off2_;
    let _e36 = sample_a((_e33 + _e34));
    let _e38 = pos_3;
    let _e39 = off2_;
    _ = (_e38 - _e39);
    let _e41 = pos_3;
    let _e42 = off2_;
    let _e44 = sample_a((_e41 - _e42));
    alpha = (((_e21 + _e28) + _e36) + _e44);
    let _e47 = fragColor_1;
    let _e48 = _e47.xyz;
    let _e49 = fragColor_1;
    let _e51 = alpha;
    outColor = vec4<f32>(_e48.x, _e48.y, _e48.z, (_e49.w * _e51));
    return;
}

@fragment 
fn main(@location(0) fragColor: vec4<f32>, @location(1) inner: f32, @location(2) pos: vec2<f32>, @location(3) off: vec2<f32>) -> FragmentOutput {
    fragColor_1 = fragColor;
    inner_1 = inner;
    pos_3 = pos;
    off_1 = off;
    main_1();
    let _e19 = outColor;
    return FragmentOutput(_e19);
}
