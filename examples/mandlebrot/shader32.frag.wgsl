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
    cd = _e8;
    let _e10 = global.alpha;
    let _e12 = cd;
    let _e15 = global.alpha;
    let _e17 = cd;
    let _e21 = global.alpha;
    let _e23 = cd;
    let _e26 = global.alpha;
    let _e28 = cd;
    let _e33 = global.delta;
    c = (vec2<f32>(((_e10.x * _e12.x) - (_e15.y * _e17.y)), ((_e21.x * _e23.y) + (_e26.y * _e28.x))) + _e33);
    let _e36 = c;
    z = _e36;
    i = 0;
    loop {
        let _e40 = i;
        let _e41 = global.iter;
        if !((_e40 < _e41)) {
            break;
        }
        {
            let _e47 = z;
            let _e49 = z;
            let _e52 = z;
            let _e54 = z;
            let _e58 = c;
            x = (((_e47.x * _e49.x) - (_e52.y * _e54.y)) + _e58.x);
            let _e62 = z;
            let _e64 = z;
            let _e67 = z;
            let _e69 = z;
            let _e73 = c;
            y = (((_e62.y * _e64.x) + (_e67.x * _e69.y)) + _e73.y);
            let _e77 = x;
            let _e78 = x;
            let _e80 = y;
            let _e81 = y;
            if (((_e77 * _e78) + (_e80 * _e81)) > 4.0) {
                break;
            }
            let _e87 = x;
            z.x = _e87;
            let _e89 = y;
            z.y = _e89;
        }
        continuing {
            let _e44 = i;
            i = (_e44 + 1);
        }
    }
    let _e90 = i;
    let _e91 = global.iter;
    if (_e90 == _e91) {
        local = 0.0;
    } else {
        let _e94 = i;
        let _e96 = global.iter;
        local = (f32(_e94) / f32(_e96));
    }
    let _e100 = local;
    r = _e100;
    let _e102 = r;
    let _e103 = r;
    g = (_e102 * _e103);
    let _e106 = g;
    let _e107 = g;
    b = (_e106 * _e107);
    let _e110 = r;
    let _e111 = g;
    let _e112 = b;
    outColor = vec4<f32>(_e110, _e111, _e112, 1.0);
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
