//! The float arithmetic of `CGUnit_C::UpdateAutoMove` (`0x6106b0`), 53 operations in thirteen
//! regions between `0x61075b` and `0x610d20` (`systems/objectlayer/unitmovement.md#steering-an-
//! auto-move`). Around the arithmetic it calls other code, whose results are inputs here: the
//! position virtual `[eax+0x14]`, the orientation getters `[eax+0x18]`, the speed
//! `CMovement::GetCurrentSpeed` (`0x7c4c90`), the clock `0x42c010`, `CRandom::uint32` (`0x4531e0`),
//! the angle wrap `0x455b30`, `fmod` (`0x73f90a`), the vector store `0x4549a0` and the steering
//! functions `0x610e40`, `0x6103d0`, `0x610520` and `0x610da0`. The regions are:
//!
//! | region | addresses           | computes                                                      |
//! |--------|---------------------|---------------------------------------------------------------|
//! | R1     | 0x61075b-0x6107d9   | the arrival test: `|pred - tgt|^2` against `|self - tgt|^2`   |
//! | R2     | 0x61080f-0x610823   | `dt = |delta| * 0.001`                                        |
//! | R3     | 0x610872-0x61087f   | `facing = orient + arg`                                       |
//! | R4     | 0x61088a-0x610897   | `turn_step = turn_rate * dt`                                  |
//! | R5     | 0x610915-0x610927   | the random jitter `(r - 1) * arg` against 0.07                |
//! | R6     | 0x6109ae-0x6109b4   | `orient_field + arg`                                          |
//! | R7     | 0x6109ba-0x6109c5   | `dt * pi`                                                     |
//! | R8     | 0x610a4c-0x610a52   | `facing - orient` (`fsubr`: memory less `st0`)                |
//! | R9     | 0x610aa2-0x610ae8   | `max(speed / 7, 1.0) * radius`                                |
//! | R10    | 0x610b07-0x610b0e   | `scaled^2`, the range gate                                    |
//! | R11    | 0x610bcf-0x610c16   | `max(t / 7, 1.0) * (3 * 1.5)`, then its square                |
//! | R12    | 0x610c85-0x610c94   | `cos(pi/180)`, the cached arrival angle threshold             |
//! | R13    | 0x610cb0-0x610d20   | the normalize of `tgt - self` and `|dz / |d||` against R12    |
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! left on the stack is not rounded; an `fst` leaves its register live, as R13's `dz` does.

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, the jitter baseline `r - 1`, the floor of the two `max(...,
/// 1.0)` clamps and the dividend of `1.0 / mag`.
pub const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `_DAT_007ffd74` (`0x00000000`): 0.0, the degenerate-magnitude compare and the `· 0.0` that drops
/// `dx·inv` and `dy·inv` from the stack while a NaN or infinity still shows.
pub const ZERO: f32 = f32::from_bits(0x0000_0000);
/// `_DAT_00801360` (`0x3a83126f`): 0.001, the milliseconds-to-seconds scale of R2.
pub const MS_TO_SEC: f32 = f32::from_bits(0x3a83_126f);
/// `_DAT_0080c608` (`0x3d8f5c29`): 0.07, the jitter threshold of R5.
pub const J5_THRESH: f32 = f32::from_bits(0x3d8f_5c29);
/// `_DAT_008030b0` (`0x40490fdb`): π, the scale of the turn step in R7.
pub const PI: f32 = f32::from_bits(0x4049_0fdb);
/// `_DAT_0080c4d0` (`0x40e00000`): 7.0, the divisor of the timers in R9 and R11.
pub const SEVEN: f32 = f32::from_bits(0x40e0_0000);
/// `_DAT_0080c4c0` (`0x40400000`): 3.0, the base of the scale in R11.
pub const THREE: f32 = f32::from_bits(0x4040_0000);
/// `_DAT_0080308c` (`0x3fc00000`): 1.5, the factor on 3.0 in R11.
pub const ONEP5: f32 = f32::from_bits(0x3fc0_0000);
/// `_DAT_0080c5f8` (the double `0x3f91df46a0000000`): π/180, the operand of `fld qword; fcos` that
/// caches `cos(1°)` at `0xc4d9d0`.
pub const DEG2RAD: f64 = f64::from_bits(0x3f91_df46_a000_0000);

/// R1 (`0x61075b` to `0x6107d9`): the self and predicted positions less the target's, each
/// component stored to a dword by `fstp [esp+..]` for `0x4549a0`, then squared and summed live as
/// `(z² + y²) + x²`. `fcompp` of `d2²` (predicted) with `d1²` (self) and `test ah,5; jp 0x6107f3`
/// fall through to the stop only on an odd count, which is `d2² < d1²`; greater, equal and NaN go
/// on. Returns `(d1², d2², stop)`.
pub fn arrival_test(self_p: [f32; 3], pred: [f32; 3], tgt: [f32; 3]) -> (f64, f64, bool) {
    let v1 = [
        (f64::from(self_p[0]) - f64::from(tgt[0])) as f32,
        (f64::from(self_p[1]) - f64::from(tgt[1])) as f32,
        (f64::from(self_p[2]) - f64::from(tgt[2])) as f32,
    ];
    let v2 = [
        (f64::from(pred[0]) - f64::from(tgt[0])) as f32,
        (f64::from(pred[1]) - f64::from(tgt[1])) as f32,
        (f64::from(pred[2]) - f64::from(tgt[2])) as f32,
    ];
    let d1 = (f64::from(v1[2]) * f64::from(v1[2]) + f64::from(v1[1]) * f64::from(v1[1]))
        + f64::from(v1[0]) * f64::from(v1[0]);
    let d2 = (f64::from(v2[2]) * f64::from(v2[2]) + f64::from(v2[1]) * f64::from(v2[1]))
        + f64::from(v2[0]) * f64::from(v2[0]);
    let stop = !d2.is_nan() && !d1.is_nan() && d2 < d1;
    (d1, d2, stop)
}

/// R2 (`0x61080f` to `0x610823`): `fild qword [ebp-0x1c]` of the 32-bit clock difference with a
/// zero high dword, `fabs; fmul 0.001; fstp dword [ebp-0x10]`.
pub fn delta_time(delta: i64) -> f32 {
    let d = delta as f64;
    (d.abs() * f64::from(MS_TO_SEC)) as f32
}

/// R3 (`0x610872` to `0x61087f`): the orientation getter `[eax+0x18]` plus `[ebp+0xc]`, stored to
/// `[ebp-0xc]`.
pub fn heading_add(orient: f32, arg: f32) -> f32 {
    (f64::from(orient) + f64::from(arg)) as f32
}

/// R4 (`0x61088a` to `0x610897`): `[0xc4d93c] · dt` stored to `[ebp-0x18]`; `0x73f90a` wraps it by
/// 2π.
pub fn turn_step(turn_rate: f32, dt: f32) -> f32 {
    (f64::from(turn_rate) * f64::from(dt)) as f32
}

/// R5 (`0x610915` to `0x610927`): `0x4531e0` gives `r = bits((rand & 0x7fffff) | 0x3f800000)` in
/// `[1, 2)`. `fld r; fsub 1.0; fmul [ebp+8]` is compared with 0.07 by `fcomp; test ah,5; jp
/// 0x610935`, which falls through only on an odd count, `(r - 1)·arg < 0.07`, and then clears bit 2
/// of the move flags. Returns whether it falls through.
pub fn rng_jitter_gate(rand_u32: u32, arg: f32) -> bool {
    let r = f32::from_bits((rand_u32 & 0x007f_ffff) | 0x3f80_0000);
    let product = (f64::from(r) - f64::from(ONE)) * f64::from(arg);
    !product.is_nan() && product < f64::from(J5_THRESH)
}

/// R6 (`0x6109ae` to `0x6109b4`): `[eax+0x20]` plus `[ebp+0xc]`, stored to `[ebp+8]`.
pub fn heading_add_field(orient_field: f32, arg: f32) -> f32 {
    (f64::from(orient_field) + f64::from(arg)) as f32
}

/// R7 (`0x6109ba` to `0x6109c5`): `dt·π` stays on the stack for the `fmod` at `0x73f90a`; this
/// returns it rounded to `f32`.
pub fn turn_step_pi(dt: f32) -> f32 {
    (f64::from(dt) * f64::from(PI)) as f32
}

/// R8 (`0x610a4c` to `0x610a52`): `fsubr [ebp-8]` is `facing - orient`, memory less `st0`, stored
/// to `[esp]`.
pub fn facing_error(orient: f32, facing: f32) -> f32 {
    (f64::from(facing) - f64::from(orient)) as f32
}

/// R9 (`0x610aa2` to `0x610ae8`): `timer / 7.0` stays on the stack. `fcomp 1.0; test ah,5; jp
/// 0x610aca` loads 1.0 on an odd count, `q < 1.0`, and otherwise divides again, so the clamp is `q`
/// for `q >= 1.0` and NaN. `fmul [0xc4d910]` and `fstp dword` give `X·radius`.
pub fn speed_from_timer(timer: f32, radius: f32) -> f32 {
    let q = f64::from(timer) / f64::from(SEVEN);
    let x = if !q.is_nan() && q < f64::from(ONE) {
        f64::from(ONE)
    } else {
        q
    };
    (x * f64::from(radius)) as f32
}

/// R10 (`0x610b07` to `0x610b0e`): `scaled²` against `goal_r2` (`[ebp-0x14]`) by `fcompp; test
/// ah,0x41; jp 0x610b29`. It falls through to `0x60f940` on an odd count, `goal_r2 <= scaled²`;
/// greater and NaN take the jump. Returns `(scaled², commit)`.
pub fn range_gate_square(scaled: f32, goal_r2: f32) -> (f64, bool) {
    let sq = f64::from(scaled) * f64::from(scaled);
    let g = f64::from(goal_r2);
    let commit = !sq.is_nan() && !g.is_nan() && g <= sq;
    (sq, commit)
}

/// R11 (`0x610bcf` to `0x610c16`): `q = time / 7.0` stays on the stack, `fld 1.0; fcomp st(1); test
/// ah,0x41; jne` replaces it with 1.0 only for `1.0 > q`, then `fld 3.0; fmul 1.5; fmulp` gives
/// `X·4.5`, and for move type 3 `fld st0; fmul st,st1` its square. Returns `(X·4.5, (X·4.5)²)`.
pub fn anim_time_speed(anim_time: f32) -> (f32, f64) {
    let q = f64::from(anim_time) / f64::from(SEVEN);
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let x = if f64::from(ONE) > q {
        f64::from(ONE)
    } else {
        q
    };
    let scaled = x * (f64::from(THREE) * f64::from(ONEP5));
    (scaled as f32, scaled * scaled)
}

/// R12 (`0x610c85` to `0x610c94`): `fld qword π/180; fcos; fstp dword [0xc4d9d0]`, the cosine of
/// one degree, computed once.
pub fn arrival_cos_threshold() -> f32 {
    DEG2RAD.cos() as f32
}

/// R13 (`0x610cb0` to `0x610d20`): `d = tgt - self`, with `dx` and `dy` stored to dwords by `fstp`
/// and `dz` by `fst`, which keeps the register. `mag² = (dx² + dy²) + dz²` where `dz²` is the live
/// `dz` times the stored one; `fcomp 0.0; test ah,0x41; jne` leaves unless `mag² > 0`. Then `inv =
/// 1.0 / sqrt(mag²)` and `zdot = |((dy·inv + dx·inv)·0.0) + dz·inv|`, and `fcomp cos(1°); test
/// ah,0x41; jne` stops only for `zdot > cos(1°)`. Returns `(zdot, mag², proceed, stop)`.
pub fn normalize_zdot(self_p: [f32; 3], tgt: [f32; 3], cos_thresh: f32) -> (f64, f64, bool, bool) {
    let dx = (f64::from(tgt[0]) - f64::from(self_p[0])) as f32;
    let dy = (f64::from(tgt[1]) - f64::from(self_p[1])) as f32;
    let dz_live = f64::from(tgt[2]) - f64::from(self_p[2]);
    let dz = dz_live as f32;
    let dx2 = f64::from(dx) * f64::from(dx);
    let dy2 = f64::from(dy) * f64::from(dy);
    let dz2 = dz_live * f64::from(dz);
    let mag2 = (dx2 + dy2) + dz2;
    let proceed = !mag2.is_nan() && mag2 > 0.0;
    if !proceed {
        return (0.0, mag2, false, false);
    }
    let mag = mag2.sqrt();
    let inv = f64::from(ONE) / mag;
    let dxi = f64::from(dx) * inv;
    let dyi = f64::from(dy) * inv;
    let dzi = f64::from(dz) * inv;
    let zdot = (((dyi + dxi) * f64::from(ZERO)) + dzi).abs();
    let stop = !zdot.is_nan() && zdot > f64::from(cos_thresh);
    (zdot, mag2, proceed, stop)
}
