//! Float kernels of the effect, equipment and unit sound code: `CEffect::Update` (`0x620580`),
//! `UpdateSpellEffectObject` (`0x620be0`), `GetDurabilityPercent` (`0x6229d0`),
//! `CGUnit_C::PlayFoley` (`0x623440`) and `CGUnit_C::GetAttachmentPoint` (`0x623b90`)
//! (`systems/objectlayer/effects.md#the-frame-tick` and `#setting-up-once-loaded`,
//! `inventory.md#searching-the-inventory`, `sounds.md#foley-and-the-alert` and
//! `#creature-reactions`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! left on the stack is not rounded. `fild` and `fidiv` read signed integers, and `__ftol`
//! (`0x40a2b0`) truncates toward zero and returns the low dword of the 64-bit result.

use crate::x87::ftol;

/// 0.001 (`0x801360`, `0x3a83126f`): the squared displacement under which `CEffect::Update` keeps
/// an effect's position.
pub const DIST2_EPS: f32 = f32::from_bits(0x3a83_126f);
/// 0.5 (`0x7ffa24`): the vector half-scale of `CEffect::Update` and `UpdateSpellEffectObject`.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// 100.0 (`0x806b10`): the durability percentage scale.
pub const HUNDRED: f32 = 100.0;
/// 2.0 (`0x801628`): the `PlayFoley` height bias.
pub const Z_BIAS_2: f32 = 2.0;

/// The re-resolve test of `CEffect::Update`, `0x620641` to `0x62067d`: after the transform query
/// `0x710600`, the deltas `newpos - cur` stay on the stack and their squared length `dx^2 + (dy^2 +
/// dz^2)` is compared with 0.001 by `fcomp; test ah,0x41; jne 0x620748`. The jump keeps the old
/// position for less, equal and NaN, so this returns whether the effect re-resolves: `dist^2 >
/// 0.001`, ordered.
pub fn effect_track_reresolve(newpos: [f32; 3], cur: [f32; 3]) -> bool {
    let dx = f64::from(newpos[0]) - f64::from(cur[0]);
    let dy = f64::from(newpos[1]) - f64::from(cur[1]);
    let dz = f64::from(newpos[2]) - f64::from(cur[2]);
    let dist2 = dx * dx + (dy * dy + dz * dz);
    let eps = f64::from(DIST2_EPS);
    !dist2.is_nan() && dist2 > eps
}

/// The half-scale of `0x6206da` to `0x620705` (and `UpdateSpellEffectObject`, `0x620d79` to
/// `0x620da5`): each component of the decomposed vector is `fld; fmul 0.5; fstp dword`.
pub fn vec3_half_scale(vec: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(vec[0]) * f64::from(HALF)) as f32,
        (f64::from(vec[1]) * f64::from(HALF)) as f32,
        (f64::from(vec[2]) * f64::from(HALF)) as f32,
    ]
}

/// The anchor add of `UpdateSpellEffectObject`, `0x620d51` to `0x620d74`: each component of the
/// local block (`[ebp-0x10]` and the next two) plus the parent block copied from `0x713640`
/// (`[ebp-0x1c]` and the next two) is `fld; fadd; fstp dword`.
pub fn effect_anchor_add(local: [f32; 3], parent: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(local[0]) + f64::from(parent[0])) as f32,
        (f64::from(local[1]) + f64::from(parent[1])) as f32,
        (f64::from(local[2]) + f64::from(parent[2])) as f32,
    ]
}

/// The scale divide of `UpdateSpellEffectObject`, `0x620f5f`: `child_scale` (`[ebp-4]`) over a
/// divisor that is the parent model scale (`[ebp-0x98]`) when it is above 0, else a model bound
/// when that is, else 1.0; `fld divisor; fld child; fdiv st,st(1); fstp dword`. The divide has no
/// guard, so a zero divisor gives an infinity or NaN.
pub fn attach_scale_div(child_scale: f32, divisor: f32) -> f32 {
    (f64::from(child_scale) / f64::from(divisor)) as f32
}

/// `GetDurabilityPercent` (`0x6229d0`): after summing `[item+0xa0]` and `[item+0xa4]` over the 19
/// equipment slots, and only when the second sum is not 0, `fild sum_current; fidiv sum_max; fmul
/// 100.0` goes through `__ftol`. Both sums are signed.
pub fn durability_percent(sum_current: i32, sum_max: i32) -> i32 {
    let ratio = f64::from(sum_current) / f64::from(sum_max);
    let scaled = ratio * f64::from(HUNDRED);
    ftol(scaled)
}

/// `CGUnit_C::PlayFoley` (`0x623440`): the sound plays two yards above the unit, `fld [ebp-4]; fadd
/// 2.0; fstp [ebp-4]` on the z of `vtable+0x14`'s position.
pub fn sound_pos_z_bias(z: f32) -> f32 {
    (f64::from(z) + f64::from(Z_BIAS_2)) as f32
}

/// The per-attachment height table at `0x862708`, 37 floats from 0.0 to 3.0.
pub const ATTACH_Z_OFFSET: [f32; 0x25] = [
    1.0, 1.0, 1.0, 1.5, 1.5, 1.8, 1.8, 0.5, 0.5, 1.0, 1.0, 2.0, 1.5, 1.0, 1.0, 1.0, 1.0, 2.0, 2.5,
    0.0, 2.0, 1.5, 1.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 3.0, 1.5, 1.5, 1.0, 1.0, 1.5, 1.0, 1.0,
];

/// `CGUnit_C::GetAttachmentPoint` (`0x623b90`), `0x623b90` onward: a model without the attachment
/// falls back to the unit's position with `table[idx]` added to z, `fld z; fadd [4*idx+0x862708];
/// fstp dword`, only for `0 <= idx < 0x25` (`test ebx,ebx; jl`, `cmp ebx,0x25; jge`); otherwise z
/// is unchanged.
pub fn sound_attach_z_bias(z: f32, idx: i32, table: &[f32]) -> f32 {
    if (0..0x25).contains(&idx) {
        (f64::from(z) + f64::from(table[idx as usize])) as f32
    } else {
        z
    }
}
