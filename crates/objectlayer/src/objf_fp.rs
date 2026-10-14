//! Float kernels of the effect, missile, sound and combat log code: `CheckEffectMatrix`
//! (`0x620930`), `CGUnit_C::PlayDeathThud` (`0x6236e0`), `CMissile::FadeOut` (`0x61d790`),
//! `CMissile::StickInUnit` (`0x61e7c0`), `CGPlayer_C::PlayVocal` (`0x62f880`),
//! `CGPlayer_C::PlayPrecastSound` (`0x62faa0`), `LogOneSidedLine` (`0x628010`) and
//! `LogTradeSkillInRange` / `LogFeedPetInRange` (`0x62d8d0`, `0x62d800`)
//! (`systems/objectlayer/effects.md#setting-up-once-loaded`, `sounds.md`, `missiles.md#fading-out`
//! and `#stuck-missiles`, `combatlog.md#the-one-sided-range-test`,
//! `combatlogmessages.md#items-pets-and-reputation`).
//!
//! The object positions come from the virtual `GetPosition(out)` (vtable `+0x14`) and are inputs.
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`; `fsqrt`
//! is `f64::sqrt`. A compare reads the live register, and the kernels that end in `fnstsw ax` return
//! the status byte the next `test ah` reads, from [`crate::x87::fcom_ah`].

use crate::x87::fcom_ah;

/// The squared magnitude of `CheckEffectMatrix` (`0x620930`), `0x620979` to `0x620998`: the first
/// row `[x, y, z]` is copied to locals, `(y² + z²) + x²` is formed on the stack, and `fst dword
/// [ebp-4]` rounds it to `f32` before the `isnan` call and the compares with 0.0.
pub fn vec3_mag2(x: f32, y: f32, z: f32) -> f32 {
    let yy = f64::from(y) * f64::from(y);
    let zz = f64::from(z) * f64::from(z);
    let xx = f64::from(x) * f64::from(x);
    ((yy + zz) + xx) as f32
}

/// `_DAT_00801628` (`0x40000000`): 2.0, the height above ground that `PlayDeathThud` compares with.
pub const LAND_HEIGHT_THRESHOLD: f32 = f32::from_bits(0x4000_0000);

/// The height test of `CGUnit_C::PlayDeathThud` (`0x6236e0`), `0x62372a` to `0x623738`: `fld
/// worldZ; fsub groundZ; fcomp 2.0`. Returns the status byte `fnstsw ax` leaves; `test ah,0x41; je
/// 0x6237ad` jumps for a height above 2.0 and falls through for equal, below and NaN.
pub fn land_height_ah(world_z: f32, ground_z: f32, threshold: f32) -> u8 {
    let delta = f64::from(world_z) - f64::from(ground_z);
    fcom_ah(delta, f64::from(threshold))
}

/// `_DAT_0080c85c` (`0x3f333334`): 0.7, the speed factor of `CMissile::FadeOut` with bit 3 of
/// `[this+0x38]` clear.
pub const PROX_SCALE_NOFLAG: f32 = f32::from_bits(0x3f33_3334);
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the speed factor with that bit set.
pub const PROX_SCALE_FLAG: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_0080297c` (`0x40400000`): 3.0, in the smoothstep `(3 - 2t)·t²`.
pub const SMOOTHSTEP_THREE: f32 = f32::from_bits(0x4040_0000);

/// The volume of `CMissile::FadeOut` (`0x61d790`), `0x61d796` to `0x61d862`, from the position `cur
/// = [this+0x2c..]`, the destination `origin = [this+0x20..]`, `speed = [this+0x1c]` and `flag8 =
/// [this+0x38] & 8`. Each difference is stored to a dword, so rounded to `f32`; the sum is `(dz² +
/// dy²) + dx²`; the speed times 0.5 or 0.7 stays on the stack. `fcompp` of `d²` with the speed
/// squared and `test ah,5; jp` leave without a call unless `d² < s²`, so `None` is returned for
/// greater, equal and NaN. Otherwise `t = sqrt(d²) / s` is stored as a dword and the call gets 0.0
/// for `t < 0`, 1.0 for `t > 1`, and `(3 - 2t)·t·t` otherwise, NaN included.
pub fn proximity_smoothstep(
    cur: [f32; 3],
    origin: [f32; 3],
    speed: f32,
    flag8: bool,
) -> Option<f32> {
    let dx: f32 = (f64::from(cur[0]) - f64::from(origin[0])) as f32;
    let dy: f32 = (f64::from(cur[1]) - f64::from(origin[1])) as f32;
    let dz: f32 = (f64::from(cur[2]) - f64::from(origin[2])) as f32;
    let scale = if flag8 {
        PROX_SCALE_FLAG
    } else {
        PROX_SCALE_NOFLAG
    };
    let scaled_speed = f64::from(speed) * f64::from(scale);
    let d2 = (f64::from(dz) * f64::from(dz) + f64::from(dy) * f64::from(dy))
        + f64::from(dx) * f64::from(dx);
    let scaled_speed2 = scaled_speed * scaled_speed;
    // `jp` is taken on an even number of C0 and C2, which leaves without the call.
    let ah_cmp1 = fcom_ah(d2, scaled_speed2) & 0x05;
    if ah_cmp1.count_ones() & 1 == 0 {
        return None;
    }
    let t: f32 = (d2.sqrt() / scaled_speed) as f32;
    // `fcomp 0.0; test ah,5; jp 0x61d81d` falls through only for C0 alone, which is `t < 0`.
    let ah0 = fcom_ah(f64::from(t), 0.0) & 0x05;
    if ah0.count_ones() & 1 == 1 {
        return Some(0.0);
    }
    // `fcomp 1.0; test ah,0x41; jne 0x61d843` falls through only for `t > 1`.
    let ah1 = fcom_ah(f64::from(t), f64::from(f32::from_bits(0x3f80_0000))) & 0x41;
    if ah1 == 0 {
        return Some(1.0);
    }
    let two_t = f64::from(t) + f64::from(t);
    let three_minus = f64::from(SMOOTHSTEP_THREE) - two_t;
    let r = (three_minus * f64::from(t) * f64::from(t)) as f32;
    Some(r)
}

/// The scale of `CMissile::StickInUnit` (`0x61e7c0`), `0x61e878` to `0x61e8a2`: `fld pick; fld b;
/// fmul c; fmul st,st1; fstp dword [esp]` gives `(b·c)·pick` with `b = [edi+0x94]`, `c =
/// [edi+0x90]` and `pick` the `[edi+0x9c]` for a non-zero `[edi+0xdc]` and `[edi+0x98]` otherwise,
/// rounded to `f32` once.
pub fn arrival_scale(b: f32, c: f32, pick_9c: f32, pick_98: f32, gate: bool) -> f32 {
    let pick = if gate { pick_9c } else { pick_98 };
    let bc = f64::from(b) * f64::from(c);
    (bc * f64::from(pick)) as f32
}

/// `_DAT_00801628` (`0x40000000`): 2.0, the height `CGPlayer_C::PlayVocal` adds to the position.
pub const SOUND_Z_BIAS: f32 = f32::from_bits(0x4000_0000);

/// The height of `CGPlayer_C::PlayVocal` (`0x62f880`), `0x62f90a` to `0x62f920`: `fld pos.z; fadd
/// 2.0; fstp dword [ebp-0xc]`.
pub fn sound_z_bias(z: f32, bias: f32) -> f32 {
    (f64::from(z) + f64::from(bias)) as f32
}

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, the height `CGPlayer_C::PlayPrecastSound` adds.
pub const SVK_Z_BIAS: f32 = f32::from_bits(0x3f80_0000);

/// The height of `CGPlayer_C::PlayPrecastSound` (`0x62faa0`), `0x62fb3c` to `0x62fb4f`: `fld pos.z;
/// fadd 1.0; fstp dword [ebp-4]`.
pub fn svk_z_bias(z: f32, bias: f32) -> f32 {
    (f64::from(z) + f64::from(bias)) as f32
}

/// The range test of `LogOneSidedLine` (`0x628010`), `0x6280ab` to `0x6280de`: `base` is the
/// target's position and `subtracted` the player's, left on the stack with the sum `(dz² + dx²) +
/// dy²`; `fcompp` compares `range·range` with it and `test ah,0x41; jne` returns 0 unless `r² >
/// d²`, so equal, less and NaN return 0.
pub fn combat_range_visible(base: [f32; 3], subtracted: [f32; 3], range: f32) -> bool {
    let d2 = dist2_dzdx(base, subtracted);
    let r2 = f64::from(range) * f64::from(range);
    r2 > d2
}

/// The range test of `LogTradeSkillInRange` (`0x62d8d0`, `0x62d957` to `0x62d98a`) and of
/// `LogFeedPetInRange` (`0x62d800`, `0x62d887` to `0x62d8ba`), whose code differs only in its call
/// operands: as [`combat_range_visible`] but the sum is `(dx² + dz²) + dy²`, and `jne` leaves
/// without the line unless `r² > d²`.
pub fn combat_defer_visible(base: [f32; 3], subtracted: [f32; 3], range: f32) -> bool {
    let d2 = dist2_dxdz(base, subtracted);
    let r2 = f64::from(range) * f64::from(range);
    r2 > d2
}

/// The sum `(dz² + dx²) + dy²` of `0x628010`, the differences left on the stack at 53 bits.
fn dist2_dzdx(base: [f32; 3], subtracted: [f32; 3]) -> f64 {
    let dx = f64::from(base[0]) - f64::from(subtracted[0]);
    let dy = f64::from(base[1]) - f64::from(subtracted[1]);
    let dz = f64::from(base[2]) - f64::from(subtracted[2]);
    (dz * dz + dx * dx) + dy * dy
}

/// The sum `(dx² + dz²) + dy²` of `0x62d8d0` and `0x62d800`.
fn dist2_dxdz(base: [f32; 3], subtracted: [f32; 3]) -> f64 {
    let dx = f64::from(base[0]) - f64::from(subtracted[0]);
    let dy = f64::from(base[1]) - f64::from(subtracted[1]);
    let dz = f64::from(base[2]) - f64::from(subtracted[2]);
    (dx * dx + dz * dz) + dy * dy
}
