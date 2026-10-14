//! Reimplements the progress ratio of `LoadTocFile` (`0x6edb90`). The x87 runs at 53-bit
//! precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in
//! `tests/toc_loader.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// 0x6edb90 — the progress-ratio (numerator+1)/denominator -> f32.
// ===========================================================================================

/// `0x6edb90`'s owned arithmetic (`0x6edd76`..`0x6edd8e`): the TOC progress fraction.
///
/// `num` is the **incremented** running count `[0xcee9fc]+1` placed in the low dword of the
/// `fild`'d qword (the high dword is forced to `0` by the source, so the i64 value is `num`
/// zero-extended — a non-negative `u32`). `den` is the total `[0xceea3c]`, the **signed** i32
/// divisor of `fidiv dword`. The PC_53 chain `(f64)(u32)num / (f64)(i32)den` is then narrowed at
/// the `fstp dword [esp]` to the `f32` handed to the progress callback. Returns that `f32`.
///
/// Op trace: `fild qword [ebp-0x24]` (exact convert); `fidiv dword [ebp-0x1c]` (the owned
/// arithmetic); `fstp dword [esp]` (narrow to f32).
pub fn progress_ratio(num: u32, den: i32) -> f32 {
    let n = f64::from(num); // fild qword (hi-dword 0 -> u32 value, exact in f64)
    let d = f64::from(den); // fidiv dword widens the signed i32 divisor exactly to f64
    (n / d) as f32 // st0 = n / d (PC_53 f64); fstp dword -> f32
}
