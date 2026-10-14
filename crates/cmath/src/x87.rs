//! The x87 float-to-integer conversions: `fistp` to a 16-, 32- or 64-bit integer and the client's
//! `__ftol` (`0x40a2b0`).
//!
//! `fistp` rounds by the control word (the client runs round to nearest, ties to even) and stores
//! the integer indefinite, the destination's most negative value (`0x8000`, `0x80000000`,
//! `0x8000000000000000`), for NaN, an infinity and a value that does not fit the destination.
//! Rust's `as` saturates and turns NaN into 0, so a transcription converts through these.
//!
//! `__ftol` sets the control word to truncate, runs `fistp qword`, and returns the qword in
//! `edx:eax`. A caller that reads `eax` takes the low dword: for a value past `i32` that is the
//! low 32 bits of the integer, and for NaN, an infinity or a value past `i64` it is 0.

const TWO_15: f64 = 32_768.0;
const TWO_31: f64 = 2_147_483_648.0;
const TWO_63: f64 = 9_223_372_036_854_775_808.0;

/// `fistp word`: round to nearest, ties to even; `0x8000` for NaN, an infinity or a value outside
/// `i16`.
#[inline]
pub fn fistp16(v: f64) -> i16 {
    let r = v.round_ties_even();
    if (-TWO_15..TWO_15).contains(&r) {
        r as i16
    } else {
        i16::MIN
    }
}

/// `fistp dword`: round to nearest, ties to even; `0x80000000` for NaN, an infinity or a value
/// outside `i32`.
#[inline]
pub fn fistp(v: f64) -> i32 {
    let r = v.round_ties_even();
    if (-TWO_31..TWO_31).contains(&r) {
        r as i32
    } else {
        i32::MIN
    }
}

/// `fistp qword`: round to nearest, ties to even; `0x8000000000000000` for NaN, an infinity or a
/// value outside `i64`.
#[inline]
pub fn fistp64(v: f64) -> i64 {
    let r = v.round_ties_even();
    if (-TWO_63..TWO_63).contains(&r) {
        r as i64
    } else {
        i64::MIN
    }
}

/// `fistp qword` with the control word set to truncate, as `__ftol` runs it: the whole 64-bit
/// result, `0x8000000000000000` for NaN, an infinity or a value outside `i64`.
#[inline]
pub fn ftol64(v: f64) -> i64 {
    let t = v.trunc();
    if (-TWO_63..TWO_63).contains(&t) {
        t as i64
    } else {
        i64::MIN
    }
}

/// `__ftol` (`0x40a2b0`) as a caller reads `eax`: the low dword of [`ftol64`].
#[inline]
pub fn ftol(v: f64) -> i32 {
    ftol64(v) as i32
}

/// `fistp dword` with the control word set to truncate: toward zero, `0x80000000` for NaN, an
/// infinity or a value outside `i32`.
#[inline]
pub fn fistp_trunc(v: f64) -> i32 {
    let t = v.trunc();
    if (-TWO_31..TWO_31).contains(&t) {
        t as i32
    } else {
        i32::MIN
    }
}
