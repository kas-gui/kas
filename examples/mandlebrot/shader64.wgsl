struct Locals {
    alpha: vec2<f64>,
    delta: vec2<f64>,
    iter: i32,
}

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<push_constant> global: Locals;

@fragment 
fn main(@location(0) cf: vec2<f32>) -> FragmentOutput {
    let ax = global.alpha.x;
    let ay = global.alpha.y;
    let iter = global.iter;
    let x = f64(cf.x);
    let y = f64(cf.y);
    let c: vec2<f64> = vec2(ax * x - ay * y, ax * y + ay * x) + global.delta;
    var z: vec2<f64> = c;
    var i: i32 = 0;

    loop {
        if !(i < iter) {
            break;
        }
        {
            let x = (z.x * z.x - z.y * z.y) + c.x;
            let y = (z.y * z.x + z.x * z.y) + c.y;

            let xf = f32(x);
            let yf = f32(y);
            if (xf * xf + yf * yf > 4.0) {
                break;
            }

            z.x = x;
            z.y = y;
        }
        continuing {
            i += 1;
        }
    }

    var r = 0.0;
    if (i != iter) {
        r = (f32(i) / f32(iter));
    }
    let g = (r * r);
    let b = (g * g);

    let outColor = vec4<f32>(r, g, b, 1.0);
    return FragmentOutput(outColor);
}
