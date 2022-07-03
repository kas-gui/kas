// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<private> col1_1: vec4<f32>;
var<private> col2_1: vec4<f32>;
var<private> pos_1: vec2<f32>;
var<private> outColor: vec4<f32>;

fn main_1() {
    var pos2_: vec2<f32>;
    var ss: f32;
    var r: f32;

    let _e4 = pos_1;
    let _e5 = pos_1;
    pos2_ = (_e4 * _e5);
    let _e8 = pos2_;
    let _e10 = pos2_;
    ss = (_e8.x + _e10.y);
    let _e14 = ss;
    if !((_e14 <= 1.0)) {
        {
            discard;
        }
    }
    _ = ss;
    let _e19 = ss;
    r = sqrt(_e19);
    _ = col1_1;
    _ = col2_1;
    _ = r;
    let _e25 = col1_1;
    let _e26 = col2_1;
    let _e27 = r;
    outColor = mix(_e25, _e26, vec4<f32>(_e27));
    return;
}

@fragment 
fn main(@location(0) @interpolate(flat) col1_: vec4<f32>, @location(1) @interpolate(flat) col2_: vec4<f32>, @location(2) pos: vec2<f32>) -> FragmentOutput {
    col1_1 = col1_;
    col2_1 = col2_;
    pos_1 = pos;
    main_1();
    let _e15 = outColor;
    return FragmentOutput(_e15);
}
