struct Locals {
    alpha: vec2<f32>,
    delta: vec2<f32>,
    iter: i32,
}

struct FragmentOutput {
    @location(0) outColor: vec4<f32>,
}

var<push_constant> global: Locals;

@fragment 
fn main(@location(0) cf: vec2<f32>) -> FragmentOutput {
    let alpha = global.alpha;
    let delta = global.delta;
    let iter = global.iter;

    var cd: vec2<f32> = cf;
    var c: vec2<f32> =
        vec2<f32>(alpha.x * cd.x - alpha.y * cd.y, alpha.x * cd.y + alpha.y * cd.x)
      + delta;
    var z: vec2<f32> = c;
    var i: i32 = 0;

    loop {
        if !(i < iter) {
            break;
        }
        {
            let x = (z.x * z.x - z.y * z.y) + c.x;
            let y = (z.y * z.x + z.x * z.y) + c.y;
            if (x * x + y * y > 4.0) {
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
