//! The float arithmetic of `CGUnit_C::EmitWaterRipple` (`0x5fa760`), 46 operations in twelve
//! regions between `0x5fa7fc` and `0x5fab67` (`systems/objectlayer/unitbody.md`). It draws a random
//! float `u` in `[1, 2)` from `CRandom::uint32` (`0x4531e0`) and bends it and the geometry into the
//! parameters of the ripple it emits and the time of the next, with its own jitter, clamps, blend
//! and attenuation. The names of the parameters (pitch, volume, pan) say what each value feeds. Each
//! region is a function here:
//!
//! | region | addresses           | computes                                                   |
//! |--------|---------------------|------------------------------------------------------------|
//! | R1     | 0x5fa7fc-0x5fa81f   | `clamped_reach = max(2 * reach, 1.0)`, ordered             |
//! | R2     | 0x5fa81f-0x5fa830   | `half = clamped_reach * 0.5`                               |
//! | R3     | 0x5fa836-0x5fa847   | `height = ground_z - self_z`; the gate `height < clamped`  |
//! | R4a    | 0x5fa89c-0x5fa8d2   | `recip = 2.5 / min(speed, 20)`                             |
//! | R4b    | 0x5fa8d4-0x5fa8e1   | `vol = 1.0 / recip`                                        |
//! | R5     | 0x5fa8ee-0x5fa8fc   | `pitch_base = orient * (1/3)`                              |
//! | R6     | 0x5fa911-0x5fa95f   | `p4 = clamp(pitch_base * (0.9 + 0.2 * (u - 1)), 1/3, 5/3)`  |
//! | R7     | 0x5fa976-0x5fa999   | `pc = 0.6 + 0.1 * (u - 1)`                                 |
//! | R8     | 0x5fa9ab-0x5fa9e6   | `p1c = (3.0 + 1.5 * (u - 1)) * 12 * (1/36) * vol`          |
//! | R9     | 0x5fa9e6-0x5faa2b   | the height blend `b = 0.5 * t + 0.5` of p14, pc and p4     |
//! | R10    | 0x5faa2f-0x5faa5f   | the material 0 scales 0.8, 1.0, 0.25 and 0.6               |
//! | R11    | 0x5faa5f-0x5faaf8   | the pan: `(u - 1) * 2pi`, or the orientation plus an offset |
//! | R12    | 0x5fab50-0x5fab67   | `dist_atten = ftol(b * recip * 0.25 * -1000)`              |
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! left on the stack is not rounded, and an `fst` leaves its register live: R3 compares the live
//! height, and R9 multiplies the live `b` for p14 but reloads the stored `b` for pc and p4.
//! `0x670630` and `0x670690` (ground), `0x672490` (the emit), `0x42c010` (the clock) and the
//! position and orientation virtuals are other code; their results are inputs.

use crate::x87::ftol;

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, the reach floor, the jitter baseline `u - 1` and a
/// recurring offset.
pub const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the half reach and the blend `0.5·t + 0.5`.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_00807a40` (`0x3eaaaaab`): 1/3 as `f32`, the pitch base scale and the lower clamp.
pub const ONE_THIRD: f32 = f32::from_bits(0x3eaa_aaab);
/// `_DAT_0080655c` (`0x3fd55555`): 5/3 as `f32`, the upper clamp of the pitch.
pub const FIVE_THIRD: f32 = f32::from_bits(0x3fd5_5555);
/// `_DAT_00804478` (`0x41a00000`): 20.0, the ceiling of the speed in `2.5 / min(speed, 20)`.
pub const SPEED_CAP: f32 = f32::from_bits(0x41a0_0000);
/// `_DAT_00803088` (`0x40200000`): 2.5, the numerator of the speed rate.
pub const RATE_NUM: f32 = f32::from_bits(0x4020_0000);
/// `_DAT_0080679c` (`0x3e4ccccd`): 0.2, the amplitude of the pitch jitter.
pub const J6_AMP: f32 = f32::from_bits(0x3e4c_cccd);
/// `_DAT_00801620` (`0x3dcccccd`): 0.1, the bias of the pitch jitter and the amplitude of the
/// second jitter.
pub const TENTH: f32 = f32::from_bits(0x3dcc_cccd);
/// `_DAT_008029c8` (`0x3d4ccccd`): 0.05, the bias of the second jitter.
pub const TWENTIETH: f32 = f32::from_bits(0x3d4c_cccd);
/// `_DAT_0080c4f0` (`0x3f266666`): 0.65, the base of the second jitter.
pub const J7_BASE: f32 = f32::from_bits(0x3f26_6666);
/// `_DAT_0080308c` (`0x3fc00000`): 1.5, the amplitude of the volume jitter.
pub const ONEP5: f32 = f32::from_bits(0x3fc0_0000);
/// `_DAT_008012cc` (`0x3f400000`): 0.75, the bias of the volume jitter.
pub const THREEQ: f32 = f32::from_bits(0x3f40_0000);
/// `_DAT_0080c4ec` (`0x40700000`): 3.75, the base of the volume jitter.
pub const J8_BASE: f32 = f32::from_bits(0x4070_0000);
/// `_DAT_0080c4e8` (`0x41400000`): 12.0, the first scale of the volume jitter.
pub const TWELVE: f32 = f32::from_bits(0x4140_0000);
/// `_DAT_007ff9d0` (`0x3ce38e39`): 1/36 as `f32`, the second scale of the volume jitter.
pub const ONE_36TH: f32 = f32::from_bits(0x3ce3_8e39);
/// `_DAT_00803568` (`0x3e2aaaab`): 1/6 as `f32`, the factor of the first blended parameter.
pub const ONE_SIXTH: f32 = f32::from_bits(0x3e2a_aaab);
/// `_DAT_0080c474` (`0x3f4ccccd`): 0.8, the material 0 scale of the first parameter.
pub const M0_S0: f32 = f32::from_bits(0x3f4c_cccd);
/// `_DAT_0080c478` (`0x3f800000`): 1.0, the material 0 scale of the second parameter.
pub const M0_S1: f32 = f32::from_bits(0x3f80_0000);
/// `_DAT_0080c47c` (`0x3e800000`): 0.25, the material 0 scale of the third parameter.
pub const QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `_DAT_0080c480` (`0x3f19999a`): 0.6, the material 0 scale of the fourth parameter.
pub const M0_S3: f32 = f32::from_bits(0x3f19_999a);
/// `_DAT_0080295c` (`0x40c90fdb`): 2π as `f32`, the scale of the random pan angle.
pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
/// `_DAT_0080c4e4` (`0x3f490fdb`): π/4, the diagonal pan offset.
pub const PI_4: f32 = f32::from_bits(0x3f49_0fdb);
/// `_DAT_0080c4e0` (`0x4016cbe4`): 3π/4, the back diagonal pan offset.
pub const PI3_4: f32 = f32::from_bits(0x4016_cbe4);
/// `_DAT_00808acc` (`0x3fc90fdb`): π/2, the side pan offset.
pub const PI_2: f32 = f32::from_bits(0x3fc9_0fdb);
/// `_DAT_008030b0` (`0x40490fdb`): π, the rear pan offset.
pub const PI: f32 = f32::from_bits(0x4049_0fdb);
/// `_DAT_008029b0` (`0x3e800000`): 0.25, a factor of the distance attenuation.
pub const R12_QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `_DAT_00801448` (`0xc47a0000`): -1000.0, the last factor of the distance attenuation.
pub const NEG_1000: f32 = f32::from_bits(0xc47a_0000);

/// R1 (`0x5fa7fc` to `0x5fa81f`): `fld [esi+0xa5c]; fadd st,st` is `2·reach`; `fcom 1.0; test
/// ah,0x41; jne` stores 1.0 for `2·reach <= 1.0` and NaN, and `fstp dword [ebp-8]` stores `2·reach`
/// only above 1.0.
pub fn clamped_reach(reach: f32) -> f32 {
    let two_reach = f64::from(reach) + f64::from(reach);
    if two_reach > f64::from(ONE) {
        two_reach as f32
    } else {
        ONE
    }
}

/// R2 (`0x5fa81f` to `0x5fa830`): `fld [ebp-8]; fmul 0.5; fstp dword [ebp-0x28]`.
pub fn half_reach(clamped_reach: f32) -> f32 {
    (f64::from(clamped_reach) * f64::from(HALF)) as f32
}

/// R3 (`0x5fa836` to `0x5fa847`): `height = ground_z - self_z` (`[ebp-0x24]` less `[eax+8]`),
/// stored to a dword by `fst [ebp-0x18]` while the register stays live. `fcomp [ebp-8]; test ah,1;
/// je 0x5fac59` bails for `height >= clamped_reach` and goes on for less and NaN; the compare reads
/// the live value. Returns the stored height and whether it goes on.
pub fn ground_height_gate(ground_z: f32, self_z: f32, clamped_reach: f32) -> (f32, bool) {
    let height_live = f64::from(ground_z) - f64::from(self_z);
    let height = height_live as f32;
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let proceed = !(height_live >= f64::from(clamped_reach));
    (height, proceed)
}

/// R4a (`0x5fa89c` to `0x5fa8d2`), reached with a material other than 0 and `speed > 1e-4`, which
/// the caller checks: `fcomp 20.0; test ah,5; jp` loads the speed when it is below 20.0 and 20.0
/// for greater, equal and NaN, and `fld 2.5; fdiv` stores `2.5 / that` as a dword at `[ebp-0x20]`.
pub fn speed_rate_recip(speed: f32) -> f32 {
    let s = f64::from(speed);
    let x = if s < f64::from(SPEED_CAP) {
        s
    } else {
        f64::from(SPEED_CAP)
    };
    (f64::from(RATE_NUM) / x) as f32
}

/// R4b (`0x5fa8d4` to `0x5fa8e1`): `fld 1.0; fdiv [ebp-0x20]; fstp dword [ebp-0x14]`.
pub fn volume_recip(recip: f32) -> f32 {
    (f64::from(ONE) / f64::from(recip)) as f32
}

/// R5 (`0x5fa8ee` to `0x5fa8fc`): the orientation the virtual `[eax+0x1c]` leaves on the stack
/// times 1/3, stored to `[ebp-4]`.
pub fn pitch_base(orient: f32) -> f32 {
    (f64::from(orient) * f64::from(ONE_THIRD)) as f32
}

/// R6 (`0x5fa911` to `0x5fa95f`): `u` is the random float in `[1, 2)` at `[ebp-0x1c]`. The jitter
/// factor is `(((u - 1)·0.2) - 0.1) + 1.0`, multiplied by `pitch_base` and stored to a dword. Then
/// `fcomp 1/3; test ah,5; jp 0x5fa948` replaces a value below 1/3 with 1/3, and `fcomp 5/3; test
/// ah,5; jnp 0x5fa95f` replaces one of 5/3 or more, and NaN, with 5/3.
pub fn pitch_jitter(u: f32, pitch_base: f32) -> f32 {
    let jit =
        (((f64::from(u) - f64::from(ONE)) * f64::from(J6_AMP)) - f64::from(TENTH)) + f64::from(ONE);
    let v = (jit * f64::from(pitch_base)) as f32;
    let vf = f64::from(v);
    if vf < f64::from(ONE_THIRD) {
        return ONE_THIRD;
    }
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(vf < f64::from(FIVE_THIRD)) {
        FIVE_THIRD
    } else {
        v
    }
}

/// R7 (`0x5fa976` to `0x5fa999`): `(((u - 1)·0.1) - 0.05) + 0.65`, stored to `[ebp-0xc]`.
pub fn jitter_pc(u: f32) -> f32 {
    let v = (((f64::from(u) - f64::from(ONE)) * f64::from(TENTH)) - f64::from(TWENTIETH))
        + f64::from(J7_BASE);
    v as f32
}

/// R8 (`0x5fa9ab` to `0x5fa9e6`): `((((u - 1)·1.5) - 0.75) + 3.75)·12·(1/36)·vol`, stored to
/// `[ebp-0x1c]`; `vol` is `[ebp-0x14]`, read before the code overwrites it.
pub fn jitter_p1c(u: f32, vol: f32) -> f32 {
    let base = (((f64::from(u) - f64::from(ONE)) * f64::from(ONEP5)) - f64::from(THREEQ))
        + f64::from(J8_BASE);
    let v = base * f64::from(TWELVE) * f64::from(ONE_36TH) * f64::from(vol);
    v as f32
}

/// R9 (`0x5fa9e6` to `0x5faa2b`): `fcomp half; test ah,0x41; jne 0x5faa2b` skips the blend for
/// `height <= half` and NaN, which keeps the inputs. Otherwise `t = (r - height) / (r - half)`
/// stays on the stack, and `b = t·0.5 + 0.5` is stored to a dword at `[ebp-0x10]` while the
/// register keeps the 53-bit value. The first parameter is that value times 1/6, and the other two
/// multiply the stored `f32` `b` by `pc` and `p4`. Returns `(p14, pc, p4, blended)`; skipped,
/// `prior_p14` goes back.
pub fn height_blend(
    height: f32,
    half: f32,
    r: f32,
    pc: f32,
    p4: f32,
    prior_p14: f32,
) -> (f32, f32, f32, bool) {
    let run = !height.is_nan() && !half.is_nan() && f64::from(height) > f64::from(half);
    if !run {
        return (prior_p14, pc, p4, false);
    }
    let t = (f64::from(r) - f64::from(height)) / (f64::from(r) - f64::from(half));
    let b_live = (t * f64::from(HALF)) + f64::from(HALF);
    let b32 = b_live as f32;
    let p14 = (b_live * f64::from(ONE_SIXTH)) as f32;
    let pc_out = (f64::from(b32) * f64::from(pc)) as f32;
    let p4_out = (f64::from(b32) * f64::from(p4)) as f32;
    (p14, pc_out, p4_out, true)
}

/// R10 (`0x5faa2f` to `0x5faa5f`), reached for material 0: each parameter is `fld const; fmul
/// param; fstp param` with the constants 0.8, 1.0, 0.25 and 0.6.
pub fn material0_scale(p14: f32, pc: f32, p1c: f32, p4: f32) -> (f32, f32, f32, f32) {
    (
        (f64::from(M0_S0) * f64::from(p14)) as f32,
        (f64::from(M0_S1) * f64::from(pc)) as f32,
        (f64::from(QUARTER) * f64::from(p1c)) as f32,
        (f64::from(M0_S3) * f64::from(p4)) as f32,
    )
}

/// R11 (`0x5faa80`), material 0: `(u - 1)·2π` stored to `[ebp+8]`.
pub fn pan_angle_random(u: f32) -> f32 {
    ((f64::from(u) - f64::from(ONE)) * f64::from(TWO_PI)) as f32
}

/// R11 (`0x5faa91` to `0x5faaf5`), material other than 0: from the movement flags `mf`
/// (`[esi+0x118]+0x40`), bit 4 adds π/4 with bit 1, 3π/4 with bit 2 and otherwise π/2 to `orient`;
/// else bit 8 subtracts the same; else bit 2 adds π; else no `fld` or `fstp` runs and `orient`
/// stays.
pub fn pan_angle_directional(orient: f32, mf: u8) -> f32 {
    let o = f64::from(orient);
    let v = if mf & 4 != 0 {
        if mf & 1 != 0 {
            o + f64::from(PI_4)
        } else if mf & 2 != 0 {
            o + f64::from(PI3_4)
        } else {
            o + f64::from(PI_2)
        }
    } else if mf & 8 != 0 {
        if mf & 1 != 0 {
            o - f64::from(PI_4)
        } else if mf & 2 != 0 {
            o - f64::from(PI3_4)
        } else {
            o - f64::from(PI_2)
        }
    } else if mf & 2 != 0 {
        o + f64::from(PI)
    } else {
        o
    };
    v as f32
}

/// R12 (`0x5fab50` to `0x5fab67`): `b·recip·0.25·-1000` stays on the stack and `__ftol` truncates
/// it toward zero; NaN or a value past `i64` gives the low dword 0. Returns `eax`.
pub fn dist_atten(b: f32, recip: f32) -> i32 {
    let prod = f64::from(b) * f64::from(recip) * f64::from(R12_QUARTER) * f64::from(NEG_1000);
    ftol(prod)
}
