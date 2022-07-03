// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

struct Locals {
    alpha: vec2<f32>,
    delta: vec2<f32>,
    iter: i32,
}

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<private> cf_1: vec2<f32>;
var<private> outColor: vec4<f32>;
var<push_constant> global: Locals;

fn main_1() {
    var cd: vec2<f32>;
    var c: vec2<f32>;
    var z: vec2<f32>;
    var i: i32;
    var x: f32;
    var y: f32;
    var local: f32;
    var r: f32;
    var g: f32;
    var b: f32;

    let _e8 = cf_1;
    cd = vec2<f32>(_e8);
    let _e11 = global.alpha;
    let _e13 = cd;
    let _e16 = global.alpha;
    let _e18 = cd;
    let _e22 = global.alpha;
    let _e24 = cd;
    let _e27 = global.alpha;
    let _e29 = cd;
    let _e34 = global.delta;
    c = (vec2<f32>(((_e11.x * _e13.x) - (_e16.y * _e18.y)), ((_e22.x * _e24.y) + (_e27.y * _e29.x))) + _e34);
    let _e37 = c;
    z = _e37;
    i = 0;
    loop {
        let _e41 = i;
        let _e42 = global.iter;
        if !((_e41 < _e42)) {
            break;
        }
        {
            let _e48 = z;
            let _e50 = z;
            let _e53 = z;
            let _e55 = z;
            let _e59 = c;
            x = (((_e48.x * _e50.x) - (_e53.y * _e55.y)) + _e59.x);
            let _e63 = z;
            let _e65 = z;
            let _e68 = z;
            let _e70 = z;
            let _e74 = c;
            y = (((_e63.y * _e65.x) + (_e68.x * _e70.y)) + _e74.y);
            let _e78 = x;
            let _e79 = x;
            let _e81 = y;
            let _e82 = y;
            if (((_e78 * _e79) + (_e81 * _e82)) > f32(4.0)) {
                break;
            }
            let _e89 = x;
            z.x = _e89;
            let _e91 = y;
            z.y = _e91;
        }
        continuing {
            let _e45 = i;
            i = (_e45 + 1);
        }
    }
    let _e92 = i;
    let _e93 = global.iter;
    if (_e92 == _e93) {
        local = 0.0;
    } else {
        let _e96 = i;
        let _e98 = global.iter;
        local = (f32(_e96) / f32(_e98));
    }
    let _e102 = local;
    r = _e102;
    let _e104 = r;
    let _e105 = r;
    g = (_e104 * _e105);
    let _e108 = g;
    let _e109 = g;
    b = (_e108 * _e109);
    let _e112 = r;
    let _e113 = g;
    let _e114 = b;
    outColor = vec4<f32>(_e112, _e113, _e114, 1.0);
    return;
}

@fragment 
fn main(@location(0) cf: vec2<f32>) -> FragmentOutput {
    cf_1 = cf;
    _ = (&global.alpha);
    _ = (&global.delta);
    main_1();
    let _e13 = outColor;
    return FragmentOutput(_e13);
}
