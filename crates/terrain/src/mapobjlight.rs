//! The map object lights the world reads: the falloff `LightFalloff` (`0x69e1c0`), the distance of
//! `GetCameraInteriorFog` (`0x69de20`), the rotated normal of `GetEntityInteriorLight`
//! (`0x69dc30`), and the alpha scale of `SplitLightColor` (`0x6a77e0`).

use crate::x87::fistp;

/// `0x69e1c0(d)` on a light with `start` at `+0x10` and `end` at `+0x14`: `d` below 0 becomes 0;
/// otherwise `d` not below `end` (NaN included) becomes `end`; then a `d` below `start` gives 1.0
/// and anything else, NaN included, `1 − (d − start)/(end − start)`, left unrounded in `st0`. Each
/// compare is `test ah,5` with `jp` or `jnp`, ordered.
pub fn wmo_molt_attenuation(start: f32, end: f32, d: f32) -> f64 {
    let d = if d < 0.0 {
        0.0
    } else if d < end {
        d
    } else {
        end
    };
    if d < start {
        return 1.0;
    }
    1.0 - (f64::from(d) - f64::from(start)) / (f64::from(end) - f64::from(start))
}

/// `0x69de20` from `0x69dfb2` to `0x69dfdc`: `√((dz² + dy²) + dx²)` with `d = light − sample`, all
/// in the register; stored as `f32` by `fst`, and compared unrounded with the light's reach.
/// Returns `(stored, compared)`.
pub fn light_distance(light: [f32; 3], sample: [f32; 3]) -> (f32, f64) {
    let [dx, dy, dz] = [0, 1, 2].map(|i| f64::from(light[i]) - f64::from(sample[i]));
    let dist = ((dz * dz + dy * dy) + dx * dx).sqrt();
    (dist as f32, dist)
}

/// The rotation `0x69dc30` reads at the definition's `+0x94`, grouped by the normal component each
/// entry multiplies.
#[derive(Clone, Copy, Debug, Default)]
pub struct InteriorLightMatrix {
    /// `+0x94`, `+0x98`, `+0x9c`: times `n2`.
    pub c0: [f32; 3],
    /// `+0xa4`, `+0xa8`, `+0xac`: times `n1`.
    pub ca: [f32; 3],
    /// `+0xb4`, `+0xb8`, `+0xbc`: times `n0`.
    pub cb: [f32; 3],
}

/// `0x69dc30` from `0x69dd03` to `0x69dd58`: the light's normal rotated by the definition's
/// matrix, `out_k = (n1·ca_k + n0·cb_k) + n2·c0_k` in the register, stored as `f32` and then at the
/// target's `+0xe4`.
pub fn interior_light_rotate(n: [f32; 3], m: &InteriorLightMatrix) -> [f32; 3] {
    let [n0, n1, n2] = n.map(f64::from);
    [0, 1, 2].map(|k| {
        let acc = n1 * f64::from(m.ca[k]) + n0 * f64::from(m.cb[k]);
        (n2 * f64::from(m.c0[k]) + acc) as f32
    })
}

/// The result of [`wmo_mocv_normalize_alpha`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MocvNormalize {
    /// The colour stored through the fourth argument.
    pub bgra: [u8; 4],
    /// The largest of b, g and r, at least 1.
    pub maxch: u8,
}

/// `0x6a77e0` with its third argument 0, so the colour branch through `0x7bbc80` and `0x7bbd60`
/// is never taken: `m = max(b, g, r)`, at least 1; when the limit is below `m`, the scale
/// `fistp(f32(limit·255/m) − 0.5)` (`0x6a7930` to `0x6a79a8`) multiplies each of b, g and r as
/// `(c·scale + 0xff) >> 8`, alpha kept; otherwise the colour is kept.
pub fn wmo_mocv_normalize_alpha(src: [u8; 4], alpha_lim: u8) -> MocvNormalize {
    let maxch = src[0].max(src[1]).max(src[2]).max(1);
    if alpha_lim >= maxch {
        return MocvNormalize { bgra: src, maxch };
    }
    let quot = ((f64::from(alpha_lim) * 255.0) / f64::from(maxch)) as f32;
    let scale = fistp(f64::from(quot) - 0.5);
    let chan = |c: u8| ((i32::from(c) * scale + 0xff) >> 8) as u8;
    MocvNormalize {
        bgra: [chan(src[0]), chan(src[1]), chan(src[2]), src[3]],
        maxch,
    }
}
