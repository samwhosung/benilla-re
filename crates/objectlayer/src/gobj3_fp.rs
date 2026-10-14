//! Float kernels of the transport code: the ship mover `0x5f50a0`, the transport handler
//! constructor `0x5f5ab0`, the transport tick `0x5f5f10` and the vector normalise `0x5f8cb0`
//! (`systems/objectlayer/transports.md#moving-the-ship` and `#the-transport-tick`,
//! `systems/objectlayer/gameobjects.md#handler-types-11-to-20`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! spilled to a `dword` and reloaded is rounded at the spill; one left on the stack is not. `fild
//! qword` reads the 64 bits it is given.

use crate::objclass_fp::normalize_vec2;
use crate::x87::fcom_ah;

/// `_DAT_00801360` (`0x3a83126f`): 0.001, the milliseconds-to-seconds factor.
pub const MS_SCALE: f32 = f32::from_bits(0x3a83_126f);
/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, the upper clamp and the reciprocal's numerator.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffd74` (`0x00000000`): 0.0, the lower clamp.
pub const ZERO: f32 = 0.0;
/// `_DAT_008026bc` (`0x35800000`): 2^-20, the squared displacement below which the transport does
/// not move.
pub const EPS_2EM20: f32 = f32::from_bits(0x3580_0000);

/// Whether `test ah,5; jp` jumps after `fcom` of `a` against `b`: the parity flag is set when the
/// masked status byte holds an even number of bits, which is every outcome but `a < b`.
fn jp_taken(a: f64, b: f64) -> bool {
    (fcom_ah(a, b) & 0x05).count_ones().is_multiple_of(2)
}

/// The position sum of `0x5f5ab0` (`0x5f5bce` to `0x5f5bf1`) and `0x5f5f10` (`0x5f5f47` to
/// `0x5f5f66`): each component is `fld base; fadd off; fstp dword`, rounded to `f32` before the
/// next.
pub fn transport_pos_add(base: [f32; 3], off: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(base[0]) + f64::from(off[0])) as f32,
        (f64::from(base[1]) + f64::from(off[1])) as f32,
        (f64::from(base[2]) + f64::from(off[2])) as f32,
    ]
}

/// The leg times of `0x5f50a0`, `0x5f51b7` to `0x5f51eb`: `fild qword` of the millisecond
/// difference (the code stores a `dword` and a zero high dword, so its range is 0 to 2^32 - 1),
/// `fmul 0.001`, `fstp dword` as an argument of `0x5f8dc0`.
pub fn transport_ms_scale(units: i64) -> f32 {
    (units as f64 * f64::from(MS_SCALE)) as f32
}

/// The leg fraction of `0x5f50a0`, `0x5f51f3` to `0x5f5222`: `(easing + addend) / divisor` on the
/// stack, `easing` being what `0x5f8dc0` returns, `addend` `[ebp-4]` and `divisor` `[ecx+esi+8]`.
/// `fcom 0.0; test ah,5; jp` stores 0.0 for a fraction below 0; `fcom 1.0; test ah,5; jp` stores
/// 1.0 for one at or above 1; both jump for NaN, which gives 1.0. Otherwise `fstp dword` stores the
/// fraction, rounded to `f32` there.
pub fn transport_leg_fraction(easing: f32, addend: f32, divisor: f32) -> f32 {
    let frac = (f64::from(easing) + f64::from(addend)) / f64::from(divisor);
    if !jp_taken(frac, f64::from(ZERO)) {
        ZERO
    } else if jp_taken(frac, f64::from(ONE)) {
        ONE
    } else {
        frac as f32
    }
}

/// The heading of `0x5f50a0`, `0x5f5251` to `0x5f52ab`: the two deltas `a` and `b` are negated and
/// stored as dwords, `0x5f8cb0` normalises the pair in place, and `fpatan` of `y` over `x` is
/// stored as the `dword` at `[edi+0x50]`. Returns the pair and the angle.
pub fn transport_heading(a: f32, b: f32) -> ([f32; 2], f32) {
    let pair = normalize_vec2((-f64::from(a)) as f32, (-f64::from(b)) as f32);
    let angle = f64::from(pair[1]).atan2(f64::from(pair[0])) as f32;
    (pair, angle)
}

/// The displacement of `0x5f5f10`, `0x5f5f6e` to `0x5f5f90`: each component of `newpos` less `cur`
/// is `fld; fsub; fstp dword`.
pub fn move_delta_sub(newpos: [f32; 3], cur: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(newpos[0]) - f64::from(cur[0])) as f32,
        (f64::from(newpos[1]) - f64::from(cur[1])) as f32,
        (f64::from(newpos[2]) - f64::from(cur[2])) as f32,
    ]
}

/// The squared length and the movement gate of `0x5f5f10`, `0x5f5f98` to `0x5f5fbe`, over the
/// displacement `(dx, dy, dz)` at `[ebp-0x18]`: the sum is `(dy² + dz²) + dx²`, `fst dword [ebp+8]`
/// stores it and keeps it on the stack at 53 bits, and `fabs; fcomp 2^-20; test ah,5; jp` is taken
/// for `|d²| >= 2^-20` and for NaN, which moves the transport. Returns the stored squared length
/// and whether the jump is taken.
pub fn move_dist2_gate(delta: [f32; 3]) -> (f32, bool) {
    let dx = f64::from(delta[0]);
    let dy = f64::from(delta[1]);
    let dz = f64::from(delta[2]);
    let dist2 = (dy * dy + dz * dz) + dx * dx;
    (dist2 as f32, jp_taken(dist2.abs(), f64::from(EPS_2EM20)))
}

/// The unit direction of `0x5f5f10`, `0x5f60f2` to `0x5f6127`: `fsqrt` of the stored squared length
/// is spilled to a dword, so `len` is rounded to `f32` before `1.0 / len`; then `dx`, `dy` and `dz`
/// are each multiplied by it and stored as dwords.
pub fn move_normalize(delta: [f32; 3], dist2: f32) -> [f32; 3] {
    let len = f64::from(dist2).sqrt() as f32;
    let inv = f64::from(ONE) / f64::from(len);
    [
        (f64::from(delta[0]) * inv) as f32,
        (f64::from(delta[1]) * inv) as f32,
        (inv * f64::from(delta[2])) as f32,
    ]
}

/// The leg speed of `0x5f5f10`, `0x5f61ac` to `0x5f61bd`: `fild qword` of the count (a `dword` and
/// a zero high dword), `fmul 0.001`, and `fdivr [ebp+8]` divides `len` by that.
pub fn move_leg_speed(len: f32, count: i64) -> f32 {
    let denom = count as f64 * f64::from(MS_SCALE);
    (f64::from(len) / denom) as f32
}
