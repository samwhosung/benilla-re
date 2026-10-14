//! Float kernels of the dynamic object and corpse code: the dynamic object's animate slot
//! `0x5d52f0` and its ground light `0x5d55c0` (`systems/spell/visuals.md#area-shard-emitter-creation`),
//! and the corpse's submerged test `0x5d6540`, model-ready slot `0x5d6850`, attach centre `0x5d68c0`
//! and decal box `0x5d6fe0` (the vtable slots of `systems/objectlayer/classes.md#overrides-by-class`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! spilled to a `dword` and reloaded is rounded at the spill; one left on the stack is not. `fild
//! qword` reads the 64 bits it is given.

use crate::x87::fcom_ah;

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, the reciprocal's numerator.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the factor on the render radius.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_007ffd74` (`0x00000000`): 0.0, the comparand of the effect radius and of a bone's z.
pub const ZERO: f32 = 0.0;
/// `_DAT_00801608` (`0x3f50624dd2f1a9fc`, a double): 0.001, the model bound above which `0x5d52f0`
/// divides by it.
pub const BOUND_MIN: f64 = f64::from_bits(0x3f50_624d_d2f1_a9fc);
/// `_DAT_008029cc` (`0x44000000`): 512.0, added to the light value of `0x5d55c0`.
pub const LIGHT_BIAS: f32 = f32::from_bits(0x4400_0000);
/// `_DAT_0080abfc` (`0x3f2aaaab`): the depth, a little above 2/3, past which a corpse is submerged.
pub const SUBMERGE_DEPTH: f32 = f32::from_bits(0x3f2a_aaab);

/// The visual scale of `0x5d52f0`, `0x5d539b` to `0x5d53d2` and `0x5d53c6` to `0x5d53d2`: `fld num;
/// fdiv den; fstp dword [esi+0x194]`, with `num` the descriptor radius and `den` the model bound or
/// the spell effect radius.
pub fn dynobj_visual_scale(num: f32, den: f32) -> f32 {
    (f64::from(num) / f64::from(den)) as f32
}

/// Whether `0x5d52f0` divides by the model bound (`0x5d538b` to `0x5d5399`): `fcomp` of the bound
/// against 0.001 and `test ah,0x41; jne` fall through only for a bound above it; equal, below and
/// NaN take the spell effect record instead.
pub fn dynobj_uses_model_bound(bound: f32) -> bool {
    fcom_ah(f64::from(bound), BOUND_MIN) & 0x41 == 0
}

/// Whether `0x5d52f0` divides by the spell effect radius (`0x5d53b6` to `0x5d53c4`): `fcomp` of the
/// radius against 0.0 and `test ah,0x41; jne` fall through only for a radius above 0; the store is
/// skipped otherwise, NaN included.
pub fn dynobj_effect_radius_usable(radius: f32) -> bool {
    fcom_ah(f64::from(radius), f64::from(ZERO)) & 0x41 == 0
}

/// The light value of `0x5d55c0`, `0x5d5650` to `0x5d5670`: `fld effect; fadd 512.0; fstp dword
/// [ebp-4]`, whose bits `shr eax,0xe` then slices.
pub fn dynobj_light_bias(effect_float: f32, bias: f32) -> f32 {
    (f64::from(effect_float) + f64::from(bias)) as f32
}

/// The submerged test of `0x5d6540`, `0x5d6584` to `0x5d6595`: `fld surface; fsub pos_z; fcomp 2/3`
/// and `test ah,0x41; jne` return 1 only for a depth above the threshold; equal, below and NaN
/// return 0.
pub fn corpse_submerged(liquid_surface_z: f32, pos_z: f32, depth_threshold: f32) -> bool {
    let depth = f64::from(liquid_surface_z) - f64::from(pos_z);
    fcom_ah(depth, f64::from(depth_threshold)) & 0x41 == 0
}

/// The model-ready slot `0x5d6850`, `0x5d689b` to `0x5d68a4`: `fld [eax+0xc]; fmul 0.5; fstp dword
/// [edi+0x2b0]`, half the render sphere radius.
pub fn corpse_half_render_radius(render_sphere_radius: f32) -> f32 {
    (f64::from(render_sphere_radius) * f64::from(HALF)) as f32
}

/// The reciprocal of `0x5d68c0`, `0x5d6b27` to `0x5d6b42`: `[ebp-0xc]` is the count `edi` and
/// `[ebp-8]` a zero, so `fild qword` reads the count as unsigned; `fld 1.0; fdiv st,st(1); fstp
/// dword [ebp-4]` rounds the reciprocal to `f32`.
pub fn corpse_center_inv(count: u32) -> f32 {
    (f64::from(ONE) / f64::from(count)) as f32
}

/// The y and z components of `0x5d68c0` (`0x5d6b56` to `0x5d6b5c`, `0x5d6b5f` to `0x5d6b6b`): `fld
/// inv; fmul [sum]; fstp dword`, the sum having been stored to a dword at `0x5d6b1f` or `0x5d6b22`.
pub fn corpse_center_component(inv: f32, sum: f32) -> f32 {
    (f64::from(inv) * f64::from(sum)) as f32
}

/// The x component of `0x5d68c0`, `0x5d6b47` to `0x5d6b4c`: the sum of x is never stored, so `fld
/// inv; fmul st,st(1); fstp dword` multiplies the 53-bit sum.
pub fn corpse_center_x(inv: f32, sum_x: f64) -> f32 {
    (f64::from(inv) * sum_x) as f32
}

/// Whether `0x5d68c0` adds a bone (`0x5d6af7` to `0x5d6b04`): `fcomp` of its z against 0.0 and
/// `test ah,0x41; jnp` skip it on an odd number of C0 and C3, which is `z < 0` and `z == 0`; above
/// 0 and NaN, whose C0, C2 and C3 leave two bits, add it.
pub fn corpse_center_accepts(z: f32) -> bool {
    (fcom_ah(f64::from(z), f64::from(ZERO)) & 0x41)
        .count_ones()
        .is_multiple_of(2)
}

/// The attach centre of `0x5d68c0`, `0x5d6aca` to `0x5d6b71`, over the bones `[x, y, z]`: the three
/// sums start at 0.0 and are added in bone order at 53 bits for each accepted bone; the y and z
/// sums are stored to dwords after the loop and the x sum is not. No accepted bone gives `None`,
/// which skips the store.
pub fn corpse_attach_center(bones: &[[f32; 3]]) -> Option<[f32; 3]> {
    let (mut sum_x, mut sum_y, mut sum_z) = (0.0f64, 0.0f64, 0.0f64);
    let mut count = 0u32;
    for b in bones {
        if corpse_center_accepts(b[2]) {
            count += 1;
            sum_x += f64::from(b[0]);
            sum_y += f64::from(b[1]);
            sum_z += f64::from(b[2]);
        }
    }
    if count == 0 {
        return None;
    }
    let inv = corpse_center_inv(count);
    Some([
        corpse_center_x(inv, sum_x),
        corpse_center_component(inv, sum_y as f32),
        corpse_center_component(inv, sum_z as f32),
    ])
}

/// The half extent of `0x5d6fe0`, `0x5d6ff7` to `0x5d7001`: `fld scale_x; fmul [esi+0x2b0]` stays
/// on the stack at 53 bits through the box.
pub fn corpse_decal_half_extent(scale_x: f32, half_render_radius: f32) -> f64 {
    f64::from(scale_x) * f64::from(half_render_radius)
}

/// The decal box of `0x5d6fe0`, `0x5d700e` to `0x5d70bb`: with `s` the half extent and `2s` its sum
/// with itself, the corners are `pos + (s, s, 2s)` and `pos - (s, s, 2s)`, each `fstp dword`.
/// Returns `(hi, lo)`.
pub fn corpse_decal_box(pos: [f32; 3], s: f64) -> ([f32; 3], [f32; 3]) {
    let two_s = s + s;
    let hi = [
        (f64::from(pos[0]) + s) as f32,
        (f64::from(pos[1]) + s) as f32,
        (f64::from(pos[2]) + two_s) as f32,
    ];
    let lo = [
        (f64::from(pos[0]) - s) as f32,
        (f64::from(pos[1]) - s) as f32,
        (f64::from(pos[2]) - two_s) as f32,
    ];
    (hi, lo)
}
