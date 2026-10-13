//! The x87 conversions the models code stores through.

/// `__ftol` (`0x40a2b0`): `fistp qword` towards zero and the low dword; NaN, an infinity or a value
/// outside `i64` stores the indefinite `0x8000000000000000`, whose low dword is 0.
#[inline]
pub(crate) fn ftol(v: f64) -> i32 {
    let t = v.trunc();
    if (-9_223_372_036_854_775_808.0_f64..9_223_372_036_854_775_808.0_f64).contains(&t) {
        t as i64 as i32
    } else {
        0
    }
}

/// `floor` (`0x73ff3f`) and then `__ftol`, the pair the liquid grid maps a coordinate to a cell
/// with.
#[inline]
pub(crate) fn floor_ftol(v: f32) -> i32 {
    ftol(f64::from(v).floor())
}

/// `fistp dword` under the client's control word: round to nearest, ties to even; NaN, an
/// infinity or a value outside `i32` stores the integer indefinite `0x80000000`.
#[inline]
pub(crate) fn fistp(v: f64) -> i32 {
    let r = v.round_ties_even();
    if (-2_147_483_648.0_f64..2_147_483_648.0_f64).contains(&r) {
        r as i32
    } else {
        i32::MIN
    }
}
