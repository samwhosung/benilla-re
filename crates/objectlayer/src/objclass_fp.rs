//! Float leaves of the game object and unit classes: the transport velocity `0x5f5c40`
//! (`systems/objectlayer/transports.md#the-transport-tick`), the 2D normalise `0x5f8cb0`
//! (`#moving-the-ship`), the footprint copy of `CGUnit_C::CacheModelData` `0x5fc310`
//! (`unitanim.md#model-events`), the cast-window test `0x5fc350`
//! (`systems/spell/records.md#range-bonuses-and-modifier`), the low-health compare in
//! `CGUnit_C::Reactivate` `0x5fbd60` (`units.md#lifecycle`), the z negate in
//! `CGUnit_C::OnMoverStepped` `0x5fac70` (`unitbody.md#liquid-and-swimming`) and the chair range
//! test `0x5f5670` (`gameobjecttypes.md#chairs`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A compare
//! reads the live register, and the kernels that end in `fnstsw ax` return the status byte the next
//! `test ah` reads, from [`fcom_ah`].

use crate::x87::fcom_ah;

/// The transport velocity of `0x5f5c40`: `s = [this+0x38]` times each of `[this+0x3c]`,
/// `[this+0x40]` and `[this+0x44]`. The three products are pushed in the reverse order and `fstp`
/// pops them into `out[0]` to `out[2]`, so `out = (s·v.x, s·v.y, s·v.z)`, each rounded to `f32` at
/// its store.
pub fn scale_vec3(s: f32, v: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(s) * f64::from(v[0])) as f32,
        (f64::from(s) * f64::from(v[1])) as f32,
        (f64::from(s) * f64::from(v[2])) as f32,
    ]
}

/// `_DAT_008029d4` (`0x34800000`): 2^-22, the length below which `0x5f8cb0` leaves its vector.
pub const NORM2_EPS: f32 = f32::from_bits(0x3480_0000);

/// The in-place normalise of `0x5f8cb0` on the vector `(x, y)` at `ecx`: `len = sqrt(x² + y²)`
/// stays on the stack at 53 bits, and `fabs; fcomp 2^-22; test ah,5; jnp` skips when `|len| <
/// 2^-22`, so the vector is returned unchanged. Equal, greater and NaN take `fdivr 1.0`, and each
/// component is `inv·x` or `inv·y` stored as a dword.
pub fn normalize_vec2(x: f32, y: f32) -> [f32; 2] {
    let xx = f64::from(x) * f64::from(x);
    let yy = f64::from(y) * f64::from(y);
    let len = (xx + yy).sqrt();
    if len.abs() < f64::from(NORM2_EPS) {
        return [x, y];
    }
    let inv = 1.0_f64 / len;
    let nx = (inv * f64::from(x)) as f32;
    let ny = (inv * f64::from(y)) as f32;
    [nx, ny]
}

/// `_DAT_007ff9d0` (`0x3ce38e39`): 1/36, the scale of the footprint sizes.
pub const TRANSFORM_SCALE: f32 = f32::from_bits(0x3ce3_8e39);

/// `CGUnit_C::CacheModelData` (`0x5fc310`): from the dwords `[src+0x18]`, `[src+0x1c]`,
/// `[src+0x20]` and `[src+0x24]` of the model data it returns the four it stores. `+0x18` goes to
/// `+0xc5c` and `+0x24` to `+0xc6c` as they are; `fmul 1/36` takes `+0x20` to `+0xc64` and `+0x1c`
/// to `+0xc68`, the two crossed, each rounded to `f32` at its store.
pub fn transform_scale_copy(
    src_18: u32,
    src_1c: f32,
    src_20: f32,
    src_24: u32,
) -> (u32, f32, f32, u32) {
    let dst_c5c = src_18;
    let dst_c64 = (f64::from(src_20) * f64::from(TRANSFORM_SCALE)) as f32;
    let dst_c68 = (f64::from(src_1c) * f64::from(TRANSFORM_SCALE)) as f32;
    let dst_c6c = src_24;
    (dst_c5c, dst_c64, dst_c68, dst_c6c)
}

/// The test of `0x5fc350`: `local = [this+0xa30]` is stored to a dword, `0x7c4c90` leaves the
/// unit's current speed `timer` in `st0`, and `fld local; fadd st,st; fcompp` compares `2·local`
/// with it. `test ah,1; jne` returns 0 on C0, which is `2·local < timer` and NaN; otherwise it
/// returns 1.
pub fn cast_time_elapsed(local: f32, timer: f32) -> bool {
    let two_local = f64::from(local) + f64::from(local);
    let ft = f64::from(timer);
    let c0_set = ft.is_nan() || two_local.is_nan() || two_local < ft;
    !c0_set
}

/// `_DAT_0080679c` (`0x3e4ccccd`): 0.2, the health fraction `CGUnit_C::Reactivate` compares with.
pub const LOW_HEALTH_FRACTION: f32 = f32::from_bits(0x3e4c_cccd);

/// The compare in `CGUnit_C::Reactivate` (`0x5fbd60`), `0x5fbd8f` to `0x5fbda3`: `fild health;
/// fidiv maxhealth` and `fcomp 0.2`. Returns the status byte `fnstsw ax` leaves, which `test ah,5;
/// jp` reads; a `maxhealth` of 0 divides to infinity or NaN.
pub fn low_health_ratio_ah(health: i32, maxhealth: i32) -> u8 {
    let ratio = f64::from(health) / f64::from(maxhealth);
    let thr = f64::from(LOW_HEALTH_FRACTION);
    fcom_ah(ratio, thr)
}

/// The z negate in `CGUnit_C::OnMoverStepped` (`0x5fac70`): `fld 0.0; fsub [eax+8]; fstp dword`
/// gives `0.0 - z` rounded to `f32`, which differs from `-z` only for `z = +0.0`.
pub fn negate_z(z: f32) -> f32 {
    (0.0_f64 - f64::from(z)) as f32
}

/// The chair range test of `0x5f5670`, `0x5f56fa` to `0x5f5726`: `pos` less `data` on each axis is
/// left on the stack, the sum is `(dz² + dx²) + dy²`, and `fcomp` compares it with `threshold`
/// (`[0xc4d808]`, the square of `[0x80b0a0]`). Returns the status byte `fnstsw ax` leaves, which
/// `test ah,5; jnp` reads.
pub fn interact_range_ah(pos: [f32; 3], data: [f32; 3], threshold: f32) -> u8 {
    let dx = f64::from(pos[0]) - f64::from(data[0]);
    let dy = f64::from(pos[1]) - f64::from(data[1]);
    let dz = f64::from(pos[2]) - f64::from(data[2]);
    let sum = (dz * dz + dx * dx) + dy * dy;
    fcom_ah(sum, f64::from(threshold))
}
