//! A model's lights and lighting record (`systems/models/lights.md`): a light's direction
//! `M2LightSetDirection` (`0x71b6a0`), the record's sums `M2LightingAddAmbient` (`0x71bc70`),
//! `M2LightingAddDirectional` (`0x71bce0`) and `M2LightingAddSpecular` (`0x71bf60`), the distance
//! `LightingAddLight` (`0x71bf90`) sorts by, the light grid of `M2SceneAddLightsForPoint`
//! (`0x70a0f0`) and `M2LightLink` (`0x71bb60`), the fog `M2LightingSetFog` (`0x71c110`), the
//! harmonic constants `M2LightingBuildShConstants` (`0x71c4e0`), the dominant light
//! `M2LightingDeriveDominantLight` (`0x71c2f0`), the view-space light `M2LightingTransformToView`
//! (`0x71c160`), and the device lights `M2LightToDeviceLight` (`0x71b7c0`),
//! `M2LightingSetDeviceLights` (`0x71c730`) and `M2PackLightColor` (`0x71ca80`).

use benilla_re_cmath::transform::transform_point;

/// 2.3841858e-7 (`0x8029d4`, `0x34800000`): the least length a direction is normalized at.
pub const NORM_EPS: f32 = f32::from_bits(0x3480_0000);
/// 9.9999997e-6 (`0x80c5c8`, `0x3727c5ac`).
pub const FLOOR_EPS: f32 = f32::from_bits(0x3727_c5ac);
/// 3.5449078 (`0x81207c`, `0x4062dfc5`): the weight of the ambient term.
pub const SH_W0: f32 = f32::from_bits(0x4062_dfc5);
/// 0.05 (`0xceffec`, `0x3d4ccccd`): world units to light grid cells, stored as `0.5/10.0` by
/// `InitM2SceneHalfTenth` (`0x70dc40`).
pub const GRID_SCALE: f32 = f32::from_bits(0x3d4c_cccd);

/// A unit value as a byte, the ladder every colour pack here uses: at or below 0 (`test ah,0x41;
/// jp`) it is 0, at or above 1 (`test ah,1; jne`) 255, otherwise, NaN included, `__ftol` of
/// `v·255 + 0.5` in the register.
pub fn unit_to_byte(v: f32) -> u8 {
    let v = f64::from(v);
    if v <= 0.0 {
        0
    } else if v >= 1.0 {
        255
    } else {
        crate::x87::ftol(v * 255.0 + 0.5) as u8
    }
}

/// `0x71b6a0`'s length `√((x² + y²) + z²)`, kept in the register.
pub fn set_dir_len(dir: [f32; 3]) -> f64 {
    let [x, y, z] = dir.map(f64::from);
    ((x * x + y * y) + z * z).sqrt()
}

/// The bits `test ah,5` reads after `0x71b6a0` compares `|len|` with 2.3841858e-7: C0 when it is
/// below, C0 and C2 when it is NaN.
pub fn set_dir_gate_ah(len: f64) -> u8 {
    let alen = len.abs();
    if alen.is_nan() {
        return 0x05;
    }
    u8::from(alen < f64::from(NORM_EPS))
}

/// `0x71b6a0(dir)` (`systems/models/lights.md#light-object`): stores the direction at `+0x24` and,
/// unless its length is below 2.3841858e-7 (ordered, `jnp` at `0x71b6ec`), multiplies each
/// component by `1/len` kept in the register.
pub fn m2_light_set_direction(dir: [f32; 3]) -> [f32; 3] {
    let len = set_dir_len(dir);
    if len.abs() < f64::from(NORM_EPS) {
        return dir;
    }
    let inv = 1.0 / len;
    dir.map(|c| (inv * f64::from(c)) as f32)
}

/// `0x71bc70(dir)` (`systems/models/lights.md#spherical-harmonic-terms`): the record's sum at
/// `+0x54` plus `dir`, and the ambient terms at `+0x84`, `+0xa8` and `+0xcc` plus `dir·3.5449078`,
/// each stored as `f32`. Returns `(sum, terms)`.
pub fn m2_light_accumulate(sum: [f32; 3], sh: [f32; 3], dir: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let w = f64::from(SH_W0);
    (
        [0, 1, 2].map(|k| sum[k] + dir[k]),
        [0, 1, 2].map(|k| (f64::from(dir[k]) * w + f64::from(sh[k])) as f32),
    )
}

/// `0x71bf60(v)` (`systems/models/lights.md#adding-a-light`): the record's `+0x6c` plus `v`, each
/// stored as `f32`.
pub fn m2light_accum_vec3(cur: [f32; 3], v: [f32; 3]) -> [f32; 3] {
    [cur[0] + v[0], v[1] + cur[1], v[2] + cur[2]]
}

/// The constants of [`m2_light_add_directional`].
mod directional {
    /// 0.0721690 (`0x81209c`), 0.2126710 (`0x8120a0`) and 0.7151600 (`0x8120a4`): the luminance.
    pub const LUMA_Z: f32 = f32::from_bits(0x3d93_cd57);
    pub const LUMA_X: f32 = f32::from_bits(0x3e59_c66d);
    pub const LUMA_Y: f32 = f32::from_bits(0x3f37_14ba);
    /// 2.9567931 (`0x812080`).
    pub const SH_SCALE: f32 = f32::from_bits(0x403d_3c19);
    /// The harmonic basis constants: 0.2820948, ±0.4886025 (`0x812094`, `0x812098`), ±1.0925485
    /// (`0x812090`, `0x81208c`), 0.3153916 (`0x812088`), 0.5462742 (`0x812084`).
    pub const Y00: f32 = f32::from_bits(0x3e90_6ebb);
    pub const K1_POS: f32 = f32::from_bits(0x3efa_2a1c);
    pub const K1_NEG: f32 = f32::from_bits(0xbefa_2a1c);
    pub const K2_POS: f32 = f32::from_bits(0x3f8b_d8a1);
    pub const K2_NEG: f32 = f32::from_bits(0xbf8b_d8a1);
    pub const K20: f32 = f32::from_bits(0x3ea1_7b02);
    pub const K22: f32 = f32::from_bits(0x3f0b_d8a1);
}

/// The part of the lighting record [`m2_light_add_directional`] writes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightAccum {
    /// `+0x18` to `+0x44`: the direction times each colour channel, then times the luminance.
    pub outer: [f32; 12],
    /// `+0x48`: the colour sum.
    pub color_sum: [f32; 3],
    /// `+0x84`, `+0xa8`, `+0xcc`: nine harmonic terms a channel.
    pub sh_r: [f32; 9],
    pub sh_g: [f32; 9],
    pub sh_b: [f32; 9],
}

impl LightAccum {
    /// A record of zeros.
    pub fn zeroed() -> LightAccum {
        LightAccum {
            outer: [0.0; 12],
            color_sum: [0.0; 3],
            sh_r: [0.0; 9],
            sh_g: [0.0; 9],
            sh_b: [0.0; 9],
        }
    }
}

/// `0x71bce0(color, dir)` (`systems/models/lights.md#spherical-harmonic-terms`): a directional
/// light folded into the record. With a matrix at the record's `+0x0` the direction is rotated by
/// its 3×3 (`+0x9c`), each row summed in the register and stored. Each channel and the luminance
/// `(c.y·0.71516 + c.x·0.21267) + c.z·0.07217` (kept in the register) times the direction is added
/// to its block: the x product unrounded, y and z stored as `f32` first. The colour is added to
/// `+0x48`. The nine harmonic basis values of the negated direction, each stored as `f32`, times
/// each channel times 2.9567931 are added to the terms.
pub fn m2_light_add_directional(
    mut acc: LightAccum,
    color: [f32; 3],
    direction: [f32; 3],
    model3x3: Option<[[f32; 3]; 3]>,
) -> LightAccum {
    use directional::*;
    let d = direction.map(f64::from);
    let (dx, dy, dz) = match model3x3 {
        None => (direction[0], direction[1], direction[2]),
        Some(m) => {
            let m = m.map(|r| r.map(f64::from));
            (
                (m[0][2] * d[2] + m[0][1] * d[1] + m[0][0] * d[0]) as f32,
                (m[1][2] * d[2] + m[1][0] * d[0] + m[1][1] * d[1]) as f32,
                (m[2][2] * d[2] + m[2][0] * d[0] + m[2][1] * d[1]) as f32,
            )
        }
    };
    let (dxf, dyf, dzf) = (f64::from(dx), f64::from(dy), f64::from(dz));

    let mut outer_block = |block: usize, c: f64| {
        acc.outer[block] = (dxf * c + f64::from(acc.outer[block])) as f32;
        acc.outer[block + 1] += (c * dyf) as f32;
        acc.outer[block + 2] += (c * dzf) as f32;
    };
    outer_block(0, f64::from(color[0]));
    outer_block(3, f64::from(color[1]));
    outer_block(6, f64::from(color[2]));
    let luma = f64::from(color[1]) * f64::from(LUMA_Y)
        + f64::from(color[0]) * f64::from(LUMA_X)
        + f64::from(color[2]) * f64::from(LUMA_Z);
    outer_block(9, luma);

    acc.color_sum = [
        acc.color_sum[0] + color[0],
        color[1] + acc.color_sum[1],
        acc.color_sum[2] + color[2],
    ];

    let (bx, by, bz) = (-dxf, -dyf, -dzf);
    let basis: [f32; 9] = [
        Y00,
        (f64::from(K1_NEG) * by) as f32,
        (f64::from(K1_POS) * bz) as f32,
        (f64::from(K1_NEG) * bx) as f32,
        (by * bx * f64::from(K2_POS)) as f32,
        (bz * by * f64::from(K2_NEG)) as f32,
        ((bz * bz * 3.0 - 1.0) * f64::from(K20)) as f32,
        (bz * bx * f64::from(K2_NEG)) as f32,
        ((bx * bx - by * by) * f64::from(K22)) as f32,
    ];
    let [cr, cg, cb] = color.map(|c| f64::from(c) * f64::from(SH_SCALE));
    for (i, &b) in basis.iter().enumerate() {
        let b = f64::from(b);
        acc.sh_r[i] = (cr * b + f64::from(acc.sh_r[i])) as f32;
        acc.sh_g[i] = (cg * b + f64::from(acc.sh_g[i])) as f32;
        acc.sh_b[i] = (cb * b + f64::from(acc.sh_b[i])) as f32;
    }
    acc
}

/// `0x71bf90` (`systems/models/lights.md#adding-a-light`): a point light's squared distance from
/// the record's centre, `(dz² + dy²) + dx²` with the differences in the register, the key it keeps
/// the four nearest by.
pub fn m2_light_aggregate_dist(light_pos: [f32; 3], probe: [f32; 3]) -> f64 {
    let [dx, dy, dz] = [0, 1, 2].map(|i| f64::from(light_pos[i]) - f64::from(probe[i]));
    (dz * dz + dy * dy) + dx * dx
}

/// The four grid coordinates [`attach_light_grid_window`] passes to `floor`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridWindow {
    pub y_min: f64,
    pub y_max: f64,
    pub z_min: f64,
    pub z_max: f64,
}

/// `0x70a0f0` from `0x70a122` to `0x70a1a7` (`systems/models/lights.md#scene-light-grid`): a
/// sphere's cells on the light grid, `(c ∓ r)·0.05 ∓ 0.5` in y and z, each in the register and
/// stored as a `f64` for `floor`.
pub fn attach_light_grid_window(
    center_y: f32,
    center_z: f32,
    extent: f32,
    scale: f32,
) -> GridWindow {
    let (cy, cz, r, s) = (
        f64::from(center_y),
        f64::from(center_z),
        f64::from(extent),
        f64::from(scale),
    );
    GridWindow {
        y_min: (cy - r) * s - 0.5,
        y_max: (cy + r) * s + 0.5,
        z_min: (cz - r) * s - 0.5,
        z_max: (cz + r) * s + 0.5,
    }
}

/// `0x71bb60` at `0x71bbbc` and `0x71bbd2` (`systems/models/lights.md#scene-light-grid`): a light's
/// coordinate times the scale at `0xcf04d0` (0.05, stored by `InitM2LightHalfTenth`, `0x71ccf0`),
/// stored as a `f64` for `floor`.
pub fn grid_scale_product(scale: f32, cell_coord: f32) -> f64 {
    f64::from(scale) * f64::from(cell_coord)
}

/// What [`fade_set_range`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeRange {
    /// `+0x184`.
    pub start: f32,
    /// `+0x188`.
    pub end: f32,
    /// `+0x18c`: `1/(end − start)`.
    pub inv_range: f32,
}

/// `0x71c110(colour, start, end)` (`systems/models/lights.md#lighting-record`): the fog's start and
/// end, and `1/(end − start)` with the difference in the register, stored as `f32`.
pub fn fade_set_range(start: f32, end: f32) -> FadeRange {
    FadeRange {
        start,
        end,
        inv_range: (1.0 / (f64::from(end) - f64::from(start))) as f32,
    }
}

/// The weights of [`sh_light_pack`], at `0x8120a8` to `0x8120c4`.
mod harmonic {
    pub const SH_A8: f64 = f32::from_bits(0x3e0b_d8a1) as f64;
    pub const SH_AC: f64 = f32::from_bits(0x3e72_3882) as f64;
    pub const SH_B0: f64 = f32::from_bits(0xbe8b_d8a1) as f64;
    pub const SH_B4: f64 = f32::from_bits(0x3e8b_d8a1) as f64;
    pub const SH_B8: f64 = f32::from_bits(0x3da1_7b01) as f64;
    pub const SH_BC: f64 = f32::from_bits(0x3e90_6ebb) as f64;
    pub const SH_C0: f64 = f32::from_bits(0x3ea6_c6bd) as f64;
    pub const SH_C4: f64 = f32::from_bits(0xbea6_c6bd) as f64;
}

/// `0x71c4e0` from `0x71c4ee` to `0x71c70e` (`systems/models/lights.md#harmonic-constants`): the 27
/// shader constants at `+0xf0` from the terms at `+0x84`: each a term times a weight, stored as
/// `f32`, three of them `a·w1 − b·w2` with both products in the register. `inp(off)` is the
/// record's float at `off`.
pub fn sh_light_pack(inp: &dyn Fn(u32) -> f32) -> [f32; 27] {
    use harmonic::*;
    let s = |off: u32, w: f64| (f64::from(inp(off)) * w) as f32;
    let d =
        |a: u32, w1: f64, b: u32, w2: f64| (f64::from(inp(a)) * w1 - f64::from(inp(b)) * w2) as f32;
    [
        s(0x90, SH_C4),
        s(0x88, SH_C4),
        s(0x8c, SH_C0),
        d(0x84, SH_BC, 0x9c, SH_B8),
        s(0xb4, SH_C4),
        s(0xac, SH_C4),
        s(0xb0, SH_C0),
        d(0xa8, SH_BC, 0xc0, SH_B8),
        s(0xd8, SH_C4),
        s(0xd0, SH_C4),
        s(0xd4, SH_C0),
        d(0xcc, SH_BC, 0xe4, SH_B8),
        s(0x94, SH_B4),
        s(0x98, SH_B0),
        s(0x9c, SH_AC),
        s(0xa0, SH_B0),
        s(0xb8, SH_B4),
        s(0xbc, SH_B0),
        s(0xc0, SH_AC),
        s(0xc4, SH_B0),
        s(0xdc, SH_B4),
        s(0xe0, SH_B0),
        s(0xe4, SH_AC),
        s(0xe8, SH_B0),
        s(0xa4, SH_A8),
        s(0xc8, SH_A8),
        s(0xec, SH_A8),
    ]
}

/// What [`camera_facing_basis`] reads.
#[derive(Clone, Copy, Debug)]
pub struct CameraBasisIn {
    /// `+0x3c`: the summed direction.
    pub src_dir: [f32; 3],
    /// `+0x18`: the 3×3 the direction is weighted by.
    pub m: [f32; 9],
    /// `+0x48`: the colour sum.
    pub vec: [f32; 3],
    /// `+0x54`: the sum the result is added to.
    pub accum_prev: [f32; 3],
}

/// What [`camera_facing_basis`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraBasis {
    /// `+0x78`: the dominant direction.
    pub dir: [f32; 3],
    /// `+0x60`: the dominant colour.
    pub basis: [f32; 3],
    /// `+0x54`: the ambient colour.
    pub accum: [f32; 3],
}

/// `0x71c2f0` (`systems/models/lights.md#dominant-light`), on a record without flag 2 at `+0x14`.
/// The direction at `+0x3c` is copied to `+0x78`; when its squared length is at most 9.9999997e-6
/// or NaN (`test ah,0x41; jne` at `0x71c345`) it becomes (0, 0, −1); otherwise, unless its length
/// (stored as `f32` by `fst`) is below 2.3841858e-7, it is divided by that length through
/// `DivideVector3` (`0x4549c0`). The weighted colour `f = M·dir` (each row summed in the register),
/// `basis = f32(1.25·f32(f)) − f32(0.25·vec)` and `accum = f32((vec − f)·0.25) + prev`.
pub fn camera_facing_basis(inp: CameraBasisIn) -> CameraBasis {
    let dir0 = inp.src_dir;
    let [x, y, z] = dir0.map(f64::from);
    let l2 = (x * x + y * y) + z * z;
    let dir = if !(l2 > f64::from(FLOOR_EPS)) {
        [0.0, 0.0, -1.0]
    } else {
        let len = l2.sqrt();
        if len.abs() < f64::from(NORM_EPS) {
            dir0
        } else {
            let inv = 1.0 / f64::from(len as f32);
            dir0.map(|c| (f64::from(c) * inv) as f32)
        }
    };

    let [d0, d1, d2] = dir.map(f64::from);
    let m = inp.m.map(f64::from);
    let f = [
        ((d2 * m[2] + d1 * m[1]) + d0 * m[0]) as f32,
        ((d2 * m[5] + d1 * m[4]) + d0 * m[3]) as f32,
        ((m[8] * d2 + m[7] * d1) + m[6] * d0) as f32,
    ];
    let quarter = inp.vec.map(|v| (f64::from(v) * 0.25) as f32);
    let basis = [0, 1, 2].map(|i| ((f64::from(f[i]) * 1.25) as f32) - quarter[i]);
    let accum = [0, 1, 2]
        .map(|i| (((f64::from(inp.vec[i]) - f64::from(f[i])) * 0.25) as f32) + inp.accum_prev[i]);
    CameraBasis { dir, basis, accum }
}

/// What [`m2_light_coord_finalize`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightCoord {
    /// `+0x1a0`: the direction in view space.
    pub dir: [f32; 3],
    /// `+0x1ac`: the plane's distance.
    pub plane_d: f32,
}

/// `0x71c160` from `0x71c1d7` to `0x71c2d9` (`systems/models/lights.md#view-space-lights`): the
/// record's plane through the matrix `m`. A point on it `p = f32(−d·dir)`; the direction rotated by
/// the 3×3 (each row summed in the register and stored) and normalized unless its length is below
/// 2.3841858e-7 (ordered, `jnp` at `0x71c28a`); `p` through `TransformPoint` (`0x7bca80`); and the
/// new distance `−((dir.z·t.z + dir.y·t.y) + t.x·dir.x)`, stored as `f32`.
pub fn m2_light_coord_finalize(m: &[f32; 16], dir: [f32; 3], d: f32) -> LightCoord {
    let neg_d = -f64::from(d);
    let p = dir.map(|c| (neg_d * f64::from(c)) as f32);
    let [dx, dy, dz] = dir.map(f64::from);
    let mf = m.map(f64::from);
    let mut dir = [
        ((mf[4] * dy + mf[8] * dz) + mf[0] * dx) as f32,
        ((mf[1] * dx + mf[9] * dz) + mf[5] * dy) as f32,
        ((mf[2] * dx + mf[10] * dz) + mf[6] * dy) as f32,
    ];
    let mag = set_dir_len(dir);
    if !(mag.abs() < f64::from(NORM_EPS)) {
        let inv = 1.0 / mag;
        dir = dir.map(|c| (inv * f64::from(c)) as f32);
    }
    let t = transform_point(&p, m);
    let dot = (f64::from(dir[2]) * f64::from(t[2]) + f64::from(dir[1]) * f64::from(t[1]))
        + f64::from(t[0]) * f64::from(dir[0]);
    LightCoord {
        dir,
        plane_d: (-dot) as f32,
    }
}

/// `0x71b7c0`'s three channels from the first two triples
/// (`systems/models/lights.md#fixed-function-lights`), packed by `0x4a2c10` as `lo << 16 | mid << 8
/// | hi`, each through [`unit_to_byte`].
pub fn light_pack_triple(ch_hi: f32, ch_mid: f32, ch_lo: f32) -> u32 {
    let [hi, mid, lo] = [ch_hi, ch_mid, ch_lo].map(|c| u32::from(unit_to_byte(c)));
    (lo << 16) | (mid << 8) | hi
}

/// `0x71b7c0`'s third triple (`0x71baa5` to `0x71bac0`) and `0x71c730`'s (`0x71c91b` to
/// `0x71ca2a`): `0xff << 24 | c3 << 16 | c2 << 8 | c1`, each through [`unit_to_byte`].
pub fn light_pack_triple3(c1: f32, c2: f32, c3: f32) -> u32 {
    let [b1, b2, b3] = [c1, c2, c3].map(|c| u32::from(unit_to_byte(c)));
    (0xff << 24) | (b3 << 16) | (b2 << 8) | b1
}

/// `0x71c730` from `0x71c820` to `0x71c843`: a light's position less the model's, stored as `f32`.
pub fn driver_vec_diff(vert_pos: [f32; 3], sub: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| vert_pos[k] - sub[k])
}

/// `0x71c730` from `0x71c902` to `0x71c912`: the colour is packed unless `|x|` is below
/// 2.3841858e-7 (ordered, `test ah,5; jnp`), so a NaN packs.
pub fn driver_mag_gate(x: f32) -> bool {
    !(f64::from(x.abs()) < f64::from(NORM_EPS))
}

/// `0x71ca80(colour, seed)` (`systems/models/lights.md#fixed-function-lights`): the largest of
/// `seed` and the three channels (a channel replaces it only when larger, ordered, so a NaN never
/// does); below 9.9999997e-6 (ordered, `jp` at `0x71cad0`) the colour is 0. Otherwise the channels
/// and `seed` times `1/max` are stored as `f32`, through [`unit_to_byte`], and packed `seed << 24 |
/// r << 16 | g << 8 | b`.
pub fn m2_light_color_pack(color: [f32; 3], seed: f32) -> u32 {
    let mut max = seed;
    for &c in &color {
        if max < c {
            max = c;
        }
    }
    if max < FLOOR_EPS {
        return 0;
    }
    let scale = 1.0 / f64::from(max);
    let [b0, b1, b2, b3] = [color[0], color[1], color[2], seed]
        .map(|c| u32::from(unit_to_byte((scale * f64::from(c)) as f32)));
    (b3 << 24) | (b0 << 16) | (b1 << 8) | b2
}
