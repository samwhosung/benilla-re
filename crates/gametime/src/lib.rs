//! The game clock (`GAMETIMECBSTRUCT`): the server-synced time of day that drives the day and
//! night cycle. All x87 arithmetic is at 53-bit precision (`f64`); `f32` operands widen exactly.
#![allow(clippy::float_cmp)]

#[inline]
fn f(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// `[0x801360]`, 0.001 as `f32` (`0x3a83126f`): milliseconds to seconds.
pub const MS_TO_S_BITS: u32 = 0x3a83_126f;
/// `[0x80e460]`, 1440.0: minutes per day.
pub const MIN_PER_DAY_BITS: u32 = 0x44b4_0000;
/// `[0x804480]`, 1/1440 as `f32`.
pub const INV_MIN_PER_DAY_BITS: u32 = 0x3a36_0b61;

/// `0x643a70`: the time of day as a fraction of a day. For the clock at `ecx`,
/// `elapsed = tick() − [+0x38]` as `u32` (the `fild` reads a 64-bit integer with a zero high
/// dword), `phase = elapsed·0.001·[+0x2c] + [+0x3c]`, less 1440 while above 1440, times 1/1440.
/// The result is the unrounded `st0`, which the caller uses as it is.
pub fn day_phase(tick: u32, base: u32, scale: f32, offset: f32) -> f64 {
    let elapsed = tick.wrapping_sub(base); // fild of [elapsed:0], zero-extended
    let mut phase = (elapsed as f64) * f(MS_TO_S_BITS) * f64::from(scale) + f64::from(offset);
    let day = f(MIN_PER_DAY_BITS);
    while phase > day {
        phase -= day;
    }
    phase * f(INV_MIN_PER_DAY_BITS)
}

/// `0x6435a0(dt)`: the clock's per-frame minute accumulator, returning the new `[+0x30]`.
/// `result = dt·[+0x2c] + [+0x30]`, stored as `f32`. If `pending = [+0x34]` is not 0 and
/// `result >= 1.0`, `clamped = min(trunc(result), pending)` (`__ftol`, then `cmp; jae`, unsigned)
/// is taken from both (`fsubr`). Then `while result >= 1.0`, `result -= 1.0` and `0x643ad0`
/// advances the clock one minute without touching `result`.
pub fn minute_accum(dt: f32, scale: f32, accum: f32, pending: i32) -> f32 {
    let mut result = (f64::from(dt) * f64::from(scale) + f64::from(accum)) as f32;
    let mut e = pending;
    if e != 0 && f64::from(result) >= 1.0 {
        let t = f64::from(result).trunc() as i32; // __ftol truncates; result >= 1, so t >= 1
        let clamped = if (e as u32) >= (t as u32) { t } else { e }; // cmp edi,t; jae
        e -= clamped;
        let _ = e; // the new [+0x34], not returned
        result = (f64::from(result) - f64::from(clamped)) as f32; // fsubr
    }
    while f64::from(result) >= 1.0 {
        result = (f64::from(result) - 1.0) as f32; // fld; fsub 1.0; fstp
    }
    result
}

/// The float arithmetic of `0x643ad0`, `0x643b11` to `0x643b1e`: the new minute offset `[+0x3c]`
/// is `fild new_minute; fadd [+0x30]; fstp`, `(double)new_minute + accum` stored as `f32`. The
/// rest of the function, the next minute mod 1440, the day change, the observers and the tick
/// base, is integer work.
pub fn advance_offset(new_minute: i32, accum: f32) -> f32 {
    (f64::from(new_minute) + f64::from(accum)) as f32
}
