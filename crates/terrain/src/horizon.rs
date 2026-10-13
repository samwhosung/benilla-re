//! The horizon buffer (`systems/terrain/cull.md#horizon-buffer`, `#horizon-box-test`): 320
//! columns at `0xc7b750`, each the highest projected height drawn into it.
//! `AddHorizonOccluder` (`0x681b50`) raises it; `TestHorizonSphere` (`0x686000`) and
//! `TestHorizonBox` (`0x686180`) test against it.
//!
//! Points go through the view matrix at `0xc7b700` (`TransformPoint`, `0x7bca80`) and are divided by
//! their z. A column is `fistp(f32(64·x) − 0.5) + 0xa0`, clamped to `[0, 0x13f]`. The functions'
//! gates on the render state (`0xc7b2a4 & 0x20`, `0xc7cff4` within ±0.9) are the caller's here.

use benilla_re_cmath::transform::transform_point;

/// `[0x810170]`: 64.0.
const SCALE_64: f32 = f32::from_bits(0x4280_0000);
/// `[0x86861c]`: 0.5, subtracted before `fistp`.
const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x810180]`: 2.7777777 (`0x4031c71c`), the least z of an occluder's vertex.
const Z_GATE_INSERT: f32 = f32::from_bits(0x4031_c71c);
/// `[0x80fed4]`: 50.0, the least z of a tested point.
const Z_GATE_TEST: f32 = f32::from_bits(0x4248_0000);
/// `[0x8029d4]`: 2^-22, the least radius `0x686000` tests.
const ABS_GATE: f32 = f32::from_bits(0x3480_0000);

/// The buffer's columns, 0x140.
pub const HORIZON_COLS: usize = 320;

/// 0xa0, added after the rounding.
const COL_BIAS: i32 = 0xa0;
/// The last column, 0x13f.
const COL_MAX: i32 = 0x13f;

/// A column from the stored `f32(64·x)`: `fistp(p − 0.5) + 0xa0`, wrapping.
#[inline]
fn col_of(sx_times_64: f32) -> i32 {
    fistp32(f64::from(sx_times_64) - f64::from(HALF)).wrapping_add(COL_BIAS)
}

/// `fistp dword`: round to nearest, ties to even; NaN, an infinity or a value outside `i32` stores
/// `0x80000000`.
#[inline]
fn fistp32(v: f64) -> i32 {
    let r = v.round_ties_even();
    if r.is_nan() || r >= 2_147_483_648.0 || r < -2_147_483_648.0 {
        i32::MIN
    } else {
        r as i32
    }
}

/// A gate `fcomp gate; test ah,5; jnp` fails only below the gate (C0 alone); equal, above and NaN
/// pass.
#[inline]
fn gate_fails(value: f32, gate: f32) -> bool {
    f64::from(value) < f64::from(gate)
}

/// `AddHorizonOccluder` (`0x681b50`, ecx = the vertices, edx = the indices, then the count and the
/// offset): each indexed vertex plus the offset (each sum stored) through the view matrix, `1/z`
/// kept in the register for the stored `x/z` and `y/z` (`0x681c0d`); then each edge whose ends both
/// have z at least 2.7777777 (`0x681c44`, `0x681c58`) raises every column from one end's to the
/// other's, clamped, to the smaller `y/z` (`0x681c69`, a tie or NaN taking the second); a column is
/// raised when below it (`0x681ce6`), NaN not.
pub fn horizon_insert(
    verts: &[[f32; 3]],
    indices: &[i32],
    offset: &[f32; 3],
    view: &[f32; 16],
    buf: &mut [f32; HORIZON_COLS],
) {
    let count = indices.len();
    if count == 0 {
        return;
    }
    let mut proj: Vec<[f32; 3]> = Vec::with_capacity(count);
    for &idx in &indices[..count] {
        let v = verts[idx as usize];
        let tmp = [
            (f64::from(v[0]) + f64::from(offset[0])) as f32,
            (f64::from(v[1]) + f64::from(offset[1])) as f32,
            (f64::from(v[2]) + f64::from(offset[2])) as f32,
        ];
        let p = transform_point(&tmp, view);
        let inv_z = f64::from(ONE) / f64::from(p[2]);
        let xz = (inv_z * f64::from(p[0])) as f32;
        let yz = (inv_z * f64::from(p[1])) as f32;
        proj.push([xz, yz, p[2]]);
    }
    if count < 2 {
        return;
    }
    for e in 0..count - 1 {
        let v0 = proj[e];
        let v1 = proj[e + 1];
        let (x0, y0, z0) = (v0[0], v0[1], v0[2]);
        let (x1, y1, z1) = (v1[0], v1[1], v1[2]);
        if gate_fails(z0, Z_GATE_INSERT) || gate_fails(z1, Z_GATE_INSERT) {
            continue;
        }
        let ymin = pick_min_y(y0, y1);
        let col0 = col_of((f64::from(SCALE_64) * f64::from(x0)) as f32);
        let col1 = col_of((f64::from(SCALE_64) * f64::from(x1)) as f32);
        let (mut lo, mut hi) = if col1 < col0 {
            (col1, col0)
        } else {
            (col0, col1)
        };
        if lo < 0 {
            lo = 0;
        }
        if hi > COL_MAX {
            hi = COL_MAX;
        }
        let mut c = lo;
        while c <= hi {
            if f64::from(buf[c as usize]) < f64::from(ymin) {
                buf[c as usize] = ymin;
            }
            c += 1;
        }
    }
}

/// `0x681c69` to `0x681c82`: `y0 < y1 ? y0 : y1` (`test ah,5; jp`), NaN taking `y1`.
#[inline]
fn pick_min_y(y0: f32, y1: f32) -> f32 {
    if f64::from(y0) < f64::from(y1) {
        y0
    } else {
        y1
    }
}

/// `TestHorizonSphere` (`0x686000`, ecx = the centre, edx = the flags, the radius on the stack):
/// 2 when the sphere is behind the horizon, else 0. A radius below 2^-22 returns 0. The centre goes
/// through the view matrix and `(w, w, 0)` through the matrix at `0xc7d280`; unless flag 8, a centre
/// z below 50 returns 0. With `1/z` the stored `cx/z`, `wx/z` and `wy/z`, the span runs from
/// `col(f32(cx/z − wy/z)·64)` to `col(f32(wy/z + cx/z)·64) + 1`; one starting at 320 or ending below
/// 0 returns 0. The value is `t = f32(cy·(1/z) + wx/z)`; the first column below it returns 0 (NaN
/// does not), and 2 when none is.
pub fn horizon_span_test(
    center: &[f32; 3],
    w: f32,
    flags: u32,
    view: &[f32; 16],
    radius_mat: &[f32; 16],
    buf: &[f32; HORIZON_COLS],
) -> u32 {
    if gate_fails(w.abs(), ABS_GATE) {
        return 0;
    }
    let c = transform_point(center, view);
    let r = transform_point(&[w, w, 0.0], radius_mat);
    let cz = c[2];
    if flags & 8 == 0 && gate_fails(cz, Z_GATE_TEST) {
        return 0;
    }
    let inv_z = f64::from(ONE) / f64::from(cz);
    let cx_z = (inv_z * f64::from(c[0])) as f32;
    let rx_z = (inv_z * f64::from(r[0])) as f32;
    let ry_z = (inv_z * f64::from(r[1])) as f32;
    let t = (f64::from(c[1]) * inv_z + f64::from(rx_z)) as f32;

    let lo_x = (f64::from(cx_z) - f64::from(ry_z)) as f32;
    let mut col_lo = col_of((f64::from(lo_x) * f64::from(SCALE_64)) as f32);
    let hi_x = (f64::from(ry_z) + f64::from(cx_z)) as f32;
    let mut col_hi = col_of_a1((f64::from(hi_x) * f64::from(SCALE_64)) as f32);

    if col_lo >= 0x140 || col_hi < 0 {
        return 0;
    }
    if col_lo < 0 {
        col_lo = 0;
    }
    if col_hi > COL_MAX {
        col_hi = COL_MAX;
    }
    let mut c = col_lo;
    loop {
        if col_hi < c {
            return 2;
        }
        if f64::from(buf[c as usize]) < f64::from(t) {
            return 0;
        }
        c += 1;
    }
}

/// `TestHorizonBox` (`0x686180`, ecx = the box, edx = the flags): 2 when the box is behind the
/// horizon, else 0. Its eight corners, min or max per axis from the tables at `0x868668` (z),
/// `0x868648` (y) and `0x868628` (x), go through the view matrix; unless flag 8, a corner z below 50
/// returns 0. Over the stored `x/z` and `y/z` it keeps the least x (NaN keeps) and the greatest x
/// and y; the span runs from `col(f32(64·min x))` to `col(f32(64·max x)) + 1` (`0x6862d4`,
/// `0x6862f1`), one starting at 320 or ending below 0 returns 0, and the first column below the
/// greatest y returns 0 (`0x686316`, NaN does not), else 2.
pub fn horizon_box_test(
    box6: &[f32; 6],
    flags: u32,
    view: &[f32; 16],
    buf: &[f32; HORIZON_COLS],
) -> u32 {
    const Z_SEL: [usize; 8] = [0, 0, 0, 0, 1, 1, 1, 1];
    const Y_SEL: [usize; 8] = [0, 0, 1, 1, 0, 0, 1, 1];
    const X_SEL: [usize; 8] = [0, 1, 1, 0, 0, 1, 1, 0];
    let vec = |sel: usize| -> [f32; 3] {
        let base = sel * 3;
        [box6[base], box6[base + 1], box6[base + 2]]
    };

    let mut min_x: f32 = f32::from_bits(0x7f7f_ffff);
    let mut max_x: f32 = f32::from_bits(0xff7f_ffff);
    let mut max_y: f32 = f32::from_bits(0xff7f_ffff);

    for i in 0..8usize {
        let corner = [vec(X_SEL[i])[0], vec(Y_SEL[i])[1], vec(Z_SEL[i])[2]];
        let p = transform_point(&corner, view);
        let z = p[2];
        if flags & 8 == 0 && gate_fails(z, Z_GATE_TEST) {
            return 0;
        }
        let inv_z = f64::from(ONE) / f64::from(z);
        let sx = (f64::from(p[0]) * inv_z) as f32;
        let sy = (f64::from(p[1]) * inv_z) as f32;
        if f64::from(sx) < f64::from(min_x) {
            min_x = sx;
        }
        if f64::from(max_x) < f64::from(sx) {
            max_x = sx;
        }
        if f64::from(max_y) < f64::from(sy) {
            max_y = sy;
        }
    }

    let mut col_lo = col_of((f64::from(SCALE_64) * f64::from(min_x)) as f32);
    let mut col_hi = col_of_a1((f64::from(SCALE_64) * f64::from(max_x)) as f32);

    if col_lo >= 0x140 || col_hi < 0 {
        return 0;
    }
    if col_lo < 0 {
        col_lo = 0;
    }
    if col_hi > COL_MAX {
        col_hi = COL_MAX;
    }
    let mut c = col_lo;
    loop {
        if col_hi < c {
            return 2;
        }
        if f64::from(buf[c as usize]) < f64::from(max_y) {
            return 0;
        }
        c += 1;
    }
}

/// The span's last column, the same rounding plus 0xa1.
#[inline]
fn col_of_a1(sx_times_64: f32) -> i32 {
    fistp32(f64::from(sx_times_64) - f64::from(HALF)).wrapping_add(COL_BIAS + 1)
}
