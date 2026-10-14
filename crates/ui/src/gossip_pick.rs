//! Reimplements the arithmetic of `0x490e60` (four rounded position legs), `0x4c0f50` (tick-to-
//! fraction band) and `PublishGossipGreeting` (`0x4e2010`, the weighted pick). The x87 runs at
//! 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests
//! in `tests/gossip_pick.rs` diff each against the exe.

#![allow(clippy::float_cmp)]
#![allow(non_snake_case)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_007ffd68` = 1024.0 (`0x44800000`), the 0x490e60 scale.
pub const SCALE1024_BITS: u32 = 0x4480_0000;
/// `DAT_00832a4c` = 1.0 (`0x3f800000`), pushed by helper 0x41ad70 (`fld [0x832a4c]`).
pub const ONE_BITS: u32 = 0x3f80_0000;
/// `DAT_00832a44` = 0.8 (`0x3f4ccccd`), the helper 0x41ae40 divisor (`fdiv [0x832a44]`).
pub const RECIP_X_BITS: u32 = 0x3f4c_cccd;
/// `DAT_007ffa24` = 0.5 (`0x3f000000`), the 0x490e60 round bias.
pub const HALF_BITS: u32 = 0x3f00_0000;
/// `DAT_00801360` = 0.001 (`0x3a83126f`), the 0x4c0f50 tick→seconds scale.
pub const MS_BITS: u32 = 0x3a83_126f;
/// `DAT_007ffe58` = 255.0 (`0x437f0000`), the 0x4c0f50 band scale.
pub const B255_BITS: u32 = 0x437f_0000;
/// `DAT_008029cc` = 512.0 (`0x44000000`), the 0x4c0f50 band offset.
pub const B512_BITS: u32 = 0x4400_0000;
/// `DAT_00801628` = 2.0 (`0x40000000`), the 0x4e2010 threshold base.
pub const TWO_BITS: u32 = 0x4000_0000;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// The CRT `__ftol` (`0x40a2b0`): `fistp qword` under round-toward-zero, returning the low dword in
/// EAX. Faithfully models the x87 64-bit conversion: in-range values truncate toward zero and EAX is
/// the low 32 bits (wrapping); out-of-i64-range or NaN yields the "integer indefinite"
/// `0x8000_0000_0000_0000`, whose low dword is `0`. This matches the binary even when the product
/// overflows i32 (the legs `push eax` = exactly this low dword).
#[inline]
fn ftol(v: f64) -> i32 {
    // The i64 range as f64 (2^63 is exactly representable; -2^63 is the inclusive min).
    const I64_MIN_F: f64 = -9_223_372_036_854_775_808.0;
    const I64_2P63: f64 = 9_223_372_036_854_775_808.0; // 2^63 (exclusive max)
    if v.is_nan() || !(I64_MIN_F..I64_2P63).contains(&v) {
        0 // integer indefinite 0x8000000000000000 → low dword 0
    } else {
        (v.trunc() as i64) as i32 // low 32 bits of the i64 truncation
    }
}

// ===========================================================================================
// (1) 0x490e60 — 4× "round-to-nearest of ((v − Y)/div · 1024) / 0.8" position legs.
// ===========================================================================================
//
// Each leg (the `[esi+0xb4]` ah-bit-gated blocks at 0x490fa2/0x49101c/0x4910a8/0x491110) computes,
// in PC_53 f64 with a single f32 round at the `fstp [esp]` mid-product:
//
//   div = [esi+0x7c]                              (a runtime field, the leg's divisor)
//   mid = (((v − Y) / div) * 1024.0) as f32       (fld 1.0; fmul 1024; fld v; [fsub Y]; fdiv div;
//                                                   fmulp; fstp [esp]  →  the f32 round)
//   x   = f64::from(mid) / 0.8                     (helper 0x41ae40: fld [esp]; fdiv [0.8])
//   out = __ftol(x ± bias)                         (legs 1,2: +0.5 if x>=0 else −0.5; legs 3,4: +0.5)
//
// Legs 1,2 carry the `fcom 0.0 / fnstsw / test ah,5 / jp` sign test (round half AWAY from zero);
// legs 3,4 unconditionally `fadd 0.5` (their inputs are non-negative by construction). Leg 1 has
// no `fsub Y` (Y term absent); legs 2,3,4 subtract a per-leg Y.

/// One 0x490e60 leg with the `fcom`-gated round-half-away-from-zero bias (legs 1 & 2). `v`, `y`,
/// `div` are the f32 inputs (`y` = 0.0 for leg 1, which has no `fsub`). Returns the `__ftol` i32.
pub fn leg_round_signed(v: f32, y: f32, div: f32) -> i32 {
    let mid = (((f64::from(v) - f64::from(y)) / f64::from(div)) * k(SCALE1024_BITS)) as f32;
    let x = f64::from(mid) / k(RECIP_X_BITS);
    // fcom [0.0]; test ah,5; jp → fadd 0.5 (x>=0, incl. +0/−0 since fcom C0 clears on −0); else fsub.
    let biased = if x >= 0.0 {
        x + k(HALF_BITS)
    } else {
        x - k(HALF_BITS)
    };
    ftol(biased)
}

/// One 0x490e60 leg with the unconditional `+0.5` bias (legs 3 & 4). Same chain as
/// [`leg_round_signed`] but always adds 0.5 (no `fcom`). Returns the `__ftol` i32.
pub fn leg_round_plus(v: f32, y: f32, div: f32) -> i32 {
    let mid = (((f64::from(v) - f64::from(y)) / f64::from(div)) * k(SCALE1024_BITS)) as f32;
    let x = f64::from(mid) / k(RECIP_X_BITS);
    ftol(x + k(HALF_BITS))
}

// ===========================================================================================
// (2) 0x4c0f50 — tick → seconds → 0..255 band, two owned FP legs around a threshold.
// ===========================================================================================

/// 0x4c0f50 leg A (`0x4c10df..0x4c10eb`): `fild qword [ebp-0xc]; fmul [0.001]; fstp [ebp-8]`. The
/// elapsed tick count (a signed i64, low dword = elapsed, high = 0) is converted and scaled by
/// 0.001 → seconds, stored as f32. `elapsed` is the i64 (signed). Returns the f32 seconds.
pub fn legA_seconds(elapsed: i64) -> f32 {
    ((elapsed as f64) * k(MS_BITS)) as f32
}

/// 0x4c0f50 leg B (`0x4c1123..0x4c113e`): `fld [ebp-8]; fdiv [D]; fmul [255]; fadd [512]; fstp
/// [ebp-8]`. The seconds value `t` (leg A's result) is normalised by the runtime denominator `d`
/// (`[0xb717bc]`), scaled to a 0..255 band and offset by 512.0 — the f32 result's bits are then
/// `>> 14` downstream (orchestration). `t` = leg A f32, `d` = denom f32. Returns the f32 band value.
pub fn legB_band(t: f32, d: f32) -> f32 {
    ((f64::from(t) / f64::from(d)) * k(B255_BITS) + k(B512_BITS)) as f32
}

// ===========================================================================================
// (3) 0x4e2010 — weighted pick over an 8-slot flag/value table.
// ===========================================================================================

/// 0x4e2010 accumulate loop (`0x4e20f5..0x4e2126`): `fld 0.0; fst [ebp-8]; for i in 0..8 { if
/// flag[i]!=0 { fadd value[i] } }; fstp [ebp-8]`. `flags`/`values` are the 8-slot arrays (`flags`
/// at `[edi]`, stride-8; `values` at `[esi]`, stride-4). The running sum is kept LIVE on st0 (PC_53
/// f64) across the loop, with the f32 round only at the closing `fstp`. Returns the f32 sum.
pub fn accumulate_sum(flags: &[bool; 8], values: &[f32; 8]) -> f32 {
    let mut acc = 0.0f64; // fld [0.0]
    for i in 0..8 {
        if flags[i] {
            acc += f64::from(values[i]);
        }
    }
    acc as f32
}

/// 0x4e2010 threshold (`0x4e212e..0x4e214b`): `fld 2.0; fsub [r]; fmul [sum]; fstp [ebp-4]`, where
/// `r` is a random float in `[1.0, 2.0)` (the caller builds it from a PRNG dword via `and 0x7fffff;
/// or 0x3f800000`). So `thr = (2.0 − r) · sum`, a uniform sample of `sum·(0,1]`. `r` is the rebuilt
/// f32, `sum` leg-1's f32. Returns the f32 threshold.
pub fn threshold(r: f32, sum: f32) -> f32 {
    ((k(TWO_BITS) - f64::from(r)) * f64::from(sum)) as f32
}

/// Rebuild the `[1.0, 2.0)` random float from a raw PRNG dword exactly as 0x4e2134:
/// `and 0x7fffff; or 0x3f800000` — keep the 23 mantissa bits, force exponent 0x7f.
pub fn rand_unit(raw: u32) -> f32 {
    f32::from_bits((raw & 0x007f_ffff) | 0x3f80_0000)
}

/// 0x4e2010 search loop (`0x4e214e..`): `acc=0.0; for i in 0..8 { if flag[i]!=0 { acc += value[i];
/// if thr <= acc { return i } } }; return 8`. Accumulates the SAME flagged values and returns the
/// first index where the running sum reaches the threshold (`fcomp` compares `thr` vs the live acc:
/// break when `thr <= acc`). `acc` stays LIVE on st0 (PC_53 f64) across iterations. Returns the
/// picked index (0..=7), or 8 if the threshold is never reached. The picked index is the function's
/// owned integer output; this models the FP `fadd`/`fcomp` that drive the branch.
pub fn weighted_pick(flags: &[bool; 8], values: &[f32; 8], thr: f32) -> usize {
    let mut acc = 0.0f64; // fld [ebp-0x14] = 0.0
    let t = f64::from(thr);
    for i in 0..8 {
        if flags[i] {
            acc += f64::from(values[i]);
            // fld [ebp-4]=thr; fcomp st(1)=acc; test ah,0x41 (C3|C0); jne → break when thr <= acc.
            if t <= acc {
                return i;
            }
        }
    }
    8
}
