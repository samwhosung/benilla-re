//! The contact triangle's class (`systems/collision/airborne.md#triangle-class-tests`): floor, a
//! ceiling the head touches, or neither.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_clip::point_in_tri;
use crate::resolve_state::aabb_top_flush_test;

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80dffc]`: cos 50°, the floor limit when `0x5fa550` returns nonzero.
const COS50: f32 = f32::from_bits(0x3f24_8dbb);
/// `[0x80e000]`: cos 80°, the limit otherwise.
const COS80: f32 = f32::from_bits(0x3e31_d0d4);

/// `CMovement::ClassifyContactTriangle` (`0x6334a0`,
/// `systems/collision/airborne.md#triangle-class-tests`): classifies a listed triangle as floor,
/// head-touching ceiling or other, clearing flag 0x4000000.
pub fn walkable_slope(
    flags: &mut u32,
    foot: &[f32; 3],
    radius: f32,
    height: f32,
    obj_pred: bool,
    normal: &[f32; 3],
    plane_d: f32,
    basis: &[[f32; 3]; 3],
    out_steep: &mut u32,
) -> u32 {
    *out_steep = 0;
    let thr = if obj_pred { COS50 } else { COS80 };
    let nz = normal[2];

    if f(nz) > f(thr) || f(nz).is_nan() {
        if point_in_tri(basis, foot) {
            *flags &= 0xfbff_ffff;
        }
        return 1;
    }

    if f(nz) >= 0.0 {
        return *flags & 0x0400_0000;
    }

    if !aabb_top_flush_test(foot, radius, height, normal, plane_d) {
        return *flags & 0x0400_0000;
    }
    if *flags & 0x0400_0000 != 0 {
        *out_steep = 1;
        *flags &= 0xfbff_ffff;
    }

    let mnz = -f(nz);
    (mnz > f(COS50) || mnz.is_nan()) as u32
}
