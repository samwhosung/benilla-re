//! The walking helpers (`systems/collision/walking.md`) and the triangle tests the airborne code
//! shares (`systems/collision/airborne.md#triangle-class-tests`).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::resolve_toolkit::Plane;

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80e014]`: −0.4756366.
const BEVEL_Z: f32 = f32::from_bits(0xbef3_86a4);
/// `[0x80e024]`: 0.6385560 (`0x3f237868`), the two-plane bisector scale.
const BISECT: f32 = f32::from_bits(0x3f23_7868);
/// `[0x8026bc]`: 2^-20.
const PAR_GUARD: f64 = 9.536_743e-7_f32 as f64;
/// `[0x8029d4]`: 2^-22.
const LEN_GUARD: f64 = 2.384_185_8e-7_f32 as f64;

/// `SelectBevelContactNormal` (`0x636610`, `systems/collision/walking.md#bevel-contact-direction`):
/// writes a contact direction from one to four bevel planes; returns 0 when a plane is not a bevel.
pub fn feature_classify(planes: &[Plane], count: u32, out: &mut [f32; 3]) -> bool {
    if count == 0 || count > 4 {
        return false;
    }
    for pl in planes.iter().take(count as usize) {
        let dz = (f(pl.n[2]) - f(BEVEL_Z)).abs();
        if !(dz < PAR_GUARD) {
            return false;
        }
    }
    match count {
        1 => *out = planes[0].n,
        2 => {
            let sx = f(planes[1].n[0]) + f(planes[0].n[0]);
            let sy = f(planes[1].n[1]) + f(planes[0].n[1]);
            let sz = (f(planes[1].n[2]) + f(planes[0].n[2])) as f32;
            out[0] = (sx * f(BISECT)) as f32;
            out[1] = (sy * f(BISECT)) as f32;
            out[2] = (f(sz) * f(BISECT)) as f32;
        }
        3 => {
            let mut list_b: Vec<usize> = Vec::new();
            let mut list_a: Vec<usize> = Vec::new();
            for (i, pl) in planes.iter().take(3).enumerate() {
                if f(pl.n[1]).abs() < PAR_GUARD {
                    list_b.push(i);
                } else if f(pl.n[0]).abs() < PAR_GUARD {
                    list_a.push(i);
                }
            }
            if list_b.len() == 1 {
                *out = planes[list_b[0]].n;
            } else {
                *out = planes[list_a[0]].n;
            }
        }
        _ => *out = [0.0, 0.0, 0.0],
    }
    true
}

/// `ComputeWallPushOut` (`0x635d80`,
/// `systems/collision/walking.md#horizontal-push-out-from-a-wall`): writes the horizontal push-out
/// that cancels a move entering a plane, plus 0.001.
pub fn vector_slide(w: &[f32; 3], speed: f32, n: &[f32; 3], out: &mut [f32; 3]) {
    const COS50: f32 = f32::from_bits(0x3f24_8dbb);
    const NUDGE: f32 = f32::from_bits(0x3a83_126f);
    // With `n.z < 0`, `−n.z` above cos 50°, or NaN, gives no push.
    if !(f(n[2]) >= 0.0) && !f(n[2]).is_nan() && !((-f(n[2])) <= f(COS50)) {
        *out = [0.0, 0.0, 0.0];
        return;
    }
    let lh = (f(n[0]) * f(n[0]) + f(n[1]) * f(n[1])).sqrt();
    let (nx, ny) = if lh.abs() < LEN_GUARD {
        (f(n[0]), f(n[1]))
    } else {
        let inv = 1.0 / lh;
        ((f(n[0]) * inv) as f32 as f64, f(n[1]) * inv)
    };
    let p = ((-f(n[2]) * f(w[2]) + -f(n[1]) * f(w[1])) + -f(n[0]) * f(w[0])) * f(speed);
    let d = (nx * f(n[0]) + ny * f(n[1])) + f(n[2]) * 0.0;
    let q = if d.abs() < LEN_GUARD { p } else { p / d } + f(NUDGE);
    out[0] = (nx * q) as f32;
    out[1] = (ny * q) as f32;
    out[2] = 0.0;
}

/// `CMovement::TryClearTransport` (`0x633200`,
/// `systems/collision/resolve.md#transport-detach-test`): returns 1 when +0x0c is nonzero and
/// 0x6173b0(0, 0) returns nonzero, else 0.
pub fn feasibility(this_c: u32, pred: bool) -> bool {
    if this_c == 0 {
        return false;
    }
    pred
}

/// `CMovement::LimitFaceRise` (`0x635c00`, `systems/collision/walking.md#rise-along-a-face`):
/// writes (0, 0, rise) for moving len along dir on a face, limiting the rise to the step height and
/// scaling len.
#[allow(clippy::too_many_arguments)]
pub fn speed_select(
    flags: u32,
    grounded_h: f32,
    w: &[f32; 3],
    speed: &mut f32,
    dir: &mut [f32; 3],
    flag: u32,
    v: &[f32; 3],
    out: &mut [f32; 3],
) {
    let v2 = ((f(v[0]) * f(v[0]) + f(v[1]) * f(v[1])) + f(v[2]) * f(v[2])) as f32;
    const COS50: f32 = f32::from_bits(0x3f24_8dbb);
    if flag != 0 {
        let skip1 = f(dir[2]) > f(COS50) || f(dir[2]).is_nan();
        if !skip1 && !(f(v2).abs() < LEN_GUARD) {
            *dir = [-v[0], -v[1], -v[2]];
        }
    }
    let p = ((-f(dir[2]) * f(w[2]) + -f(dir[1]) * f(w[1])) + -f(dir[0]) * f(w[0])) * f(*speed);
    let t: f32 = if f(dir[2]).abs() < LEN_GUARD {
        if p < 0.0 {
            f32::from_bits(0xff7f_ffff)
        } else {
            f32::from_bits(0x7f7f_ffff)
        }
    } else {
        (p / f(dir[2])) as f32
    };
    let h = grounded_h;
    let final_scalar: f32 = if flags & 0x0400_0000 != 0 && !(f(t) >= 0.0) && !f(t).is_nan() {
        *speed = 0.0;
        h
    } else if !(f(h) >= f(t)) && !f(h).is_nan() && !f(t).is_nan() {
        *speed = ((f(h) / f(t)) * f(*speed)) as f32;
        h
    } else if -f(h) > f(t) {
        *speed = (-((f(h) / f(t)) * f(*speed))) as f32;
        (-f(h)) as f32
    } else {
        t
    };
    let xy = (0.0f64 * f(final_scalar)) as f32;
    *out = [xy, xy, final_scalar];
}

/// `IsListedTriangleSteep` (`0x6336a0`, `systems/collision/airborne.md#triangle-class-tests`):
/// returns 1 when a listed triangle's normal z lies between -cos 50 degrees and cos 50 degrees.
pub fn wall_test(nz: f32) -> bool {
    const COS50: f32 = f32::from_bits(0x3f24_8dbb);
    if f(nz) > f(COS50) || f(nz).is_nan() {
        return false;
    }
    if !(f(nz) < 0.0) {
        return true;
    }
    !((-f(nz)) > f(COS50) || f(nz).is_nan())
}

/// `CMovement::LandsOnTriangle` (`0x633720`, `systems/collision/airborne.md#triangle-class-tests`):
/// returns 1 when a listed triangle is floor for the unit under its position plus a move.
pub fn ground_at_point(
    pos: &[f32; 3],
    offset: &[f32; 3],
    rec_nz: f32,
    rec_basis: &[[f32; 3]; 3],
    obj_pred: bool,
) -> bool {
    const COS50: f32 = f32::from_bits(0x3f24_8dbb);
    const COS80: f32 = f32::from_bits(0x3e31_d0d4);
    let p: [f32; 3] = [
        (f(offset[0]) + f(pos[0])) as f32,
        (f(pos[1]) + f(offset[1])) as f32,
        (f(pos[2]) + f(offset[2])) as f32,
    ];
    let gate = if obj_pred { COS50 } else { COS80 };

    let up = f(rec_nz) > f(gate) || f(rec_nz).is_nan();
    if !up {
        return false;
    }
    crate::resolve_clip::point_in_tri(rec_basis, &p)
}

/// `CMovement::ComputeWallPush` (`0x635090`, `systems/collision/airborne.md#horizontal-wall-push`):
/// writes the horizontal push that undoes a remaining move's part into a wall normal.
#[allow(clippy::too_many_arguments)]
pub fn slide_response(
    this_pos: &[f32; 3],
    radius: f32,
    dir: &[f32; 3],
    t: f32,
    len: f32,
    hit_nz: f32,
    hit_normal: &[f32; 3],
    hit_d: f32,
    hit_basis: &[[f32; 3]; 3],
    feature: &[crate::resolve_toolkit::Plane; 2],
    feature_type: u32,
    out: &mut [f32; 2],
) {
    use crate::resolve_clip::slide_normal_select;
    let n: [f32; 3] = if wall_test(hit_nz) {
        slide_normal_select(
            this_pos,
            radius,
            hit_normal,
            hit_d,
            hit_basis,
            feature,
            feature_type,
        )
    } else {
        let e0 = &feature[0].n;
        [-e0[0], -e0[1], -e0[2]]
    };
    let lh = (f(n[0]) * f(n[0]) + f(n[1]) * f(n[1])).sqrt();
    let (nx, ny) = if lh.abs() < LEN_GUARD {
        (f(n[0]), f(n[1]))
    } else {
        let inv = 1.0 / lh;
        ((f(n[0]) * inv) as f32 as f64, f(n[1]) * inv)
    };
    let p =
        (-f(n[0]) * f(dir[0]) + (-f(n[1])) * f(dir[1]) + (-f(n[2])) * f(dir[2])) * (f(len) - f(t));
    let d = (f(n[1]) * ny + nx * f(n[0])) + f(n[2]) * 0.0;
    let s = if d.abs() < LEN_GUARD { p } else { p / d };
    out[0] = (nx * s) as f32;
    out[1] = (ny * s) as f32;
}

/// `IsNotFloorAt` (`0x633760`, `systems/collision/airborne.md#triangle-class-tests`): returns 0
/// when a listed triangle is within the unit's slope limit and lies under a point, else 1.
pub fn walkable_hit(rec_nz: f32, obj_pred: bool, point_in_face: bool) -> u32 {
    const COS50: f32 = f32::from_bits(0x3f24_8dbb);
    const COS80: f32 = f32::from_bits(0x3e31_d0d4);
    let gate = if obj_pred { COS50 } else { COS80 };

    let up = f(rec_nz) > f(gate) || f(rec_nz).is_nan();
    if up && point_in_face {
        0
    } else {
        1
    }
}
