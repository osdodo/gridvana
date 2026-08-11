use crate::model::Rgba;

pub fn rgb_to_hsv(color: Rgba) -> (f32, f32, f32) {
    let r = color.r.clamp(0.0, 1.0);
    let g = color.g.clamp(0.0, 1.0);
    let b = color.b.clamp(0.0, 1.0);

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let saturation = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };

    (hue.rem_euclid(360.0), saturation.clamp(0.0, 1.0), max)
}

pub fn hsv_to_rgba(hue: f32, saturation: f32, value: f32, alpha: f32) -> Rgba {
    let hue = hue.rem_euclid(360.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);
    let chroma = value * saturation;
    let segment = hue / 60.0;
    let x = chroma * (1.0 - ((segment.rem_euclid(2.0)) - 1.0).abs());

    let (r1, g1, b1) = match segment.floor() as i32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    let m = value - chroma;

    Rgba::new(r1 + m, g1 + m, b1 + m, alpha.clamp(0.0, 1.0))
}
