//! Reimplements the arithmetic of `CCamera::Update` (`0x50e8d0`: mood fade, look rates, overshoot
//! re-centre, distance fade, shake) and of three Lua and text functions: `GetTradeSkillCooldown`
//! (`0x4fda20`), `ExpandSpellToken` (`0x507710`) and `DrunkTransformText` (`0x508680`). The x87
//! runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the
//! tests in `tests/camera_update.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants — read from WoW.exe (bit-exact; cited by VA / objdump -s -j .rdata).
// ===========================================================================================

/// `0x801360` (`.rdata`) = `0.001` as a **dword** `f32` (`0x3a83126f`, = 1/1000). The ms→s scale
/// in `0x4fda20`, and the floor/ceil epsilon in `0x507710`/`0x50e8d0`.
pub const MS_TO_S_F32_BITS: u32 = 0x3a83_126f;
/// `0x7ff9d8` (`.rdata`) = `1.0` as a **dword** `f32` (`0x3f800000`). The `/n` reciprocal numerator
/// in `0x507710`, and the `local_8 - 1.0` mantissa-bias subtrahend in `0x508680`.
pub const ONE_F32_BITS: u32 = 0x3f80_0000;
/// `0x84f2d4` (`.rdata`) = `0.5` as a **dword** `f32` (`0x3f000000`). The round-to-nearest bias in
/// `0x507710`'s minutes/seconds rounding island.
pub const HALF_84F2D4_F32_BITS: u32 = 0x3f00_0000;
/// `0x7ffa24` (`.rdata`) = `0.5` as a **dword** `f32` (`0x3f000000`). The `'f'`-case quantize bias.
pub const HALF_7FFA24_F32_BITS: u32 = 0x3f00_0000;
/// `0x8029b0` (`.rdata`) = `0.25` as a **dword** `f32` (`0x3e800000`). The `0x508680` `param_4·0.25`
/// threshold scale (the first, end-of-string flush case).
pub const QUARTER_F32_BITS: u32 = 0x3e80_0000;

// --- camera driver 0x50e8d0 fade/rate constants ---
/// `0x8029d0` = `0.01` (`0x3c23d70a`), the mood-fade byte `·0.01` scale.
pub const MOOD_FADE_F32_BITS: u32 = 0x3c23_d70a;
/// `0x7ffaac` = `π/180` (`0x3c8efa35` = 0.017453292…), the look-rate degrees→radians scale.
pub const DEG2RAD_F32_BITS: u32 = 0x3c8e_fa35;
/// `0x808a00` / `0x808a04` = `1/9` (`0x3de38e39`), the overshoot-recenter near/far back-off offset.
pub const NINTH_F32_BITS: u32 = 0x3de3_8e39;
/// `0x8026bc` = `9.5367e-07` (`0x35800000`), the overshoot-recenter post-add bias (and the
/// `_DAT_00808a00` distance-positive ε's companion).
pub const RECENTER_BIAS_F32_BITS: u32 = 0x3580_0000;
/// `0x808a94` = `-π/2` (`0xbfc90fdb` = -1.5707963…), the distance-fade ratio numerator anchor.
pub const FADE_A_F32_BITS: u32 = 0xbfc9_0fdb;
/// `0x808a98` = `-1.1868…` (`0xbf97e9d8`), the distance-fade ratio denominator anchor.
pub const FADE_B_F32_BITS: u32 = 0xbf97_e9d8;
/// `0x8089b0` = `1.8315…` (`0x3fea6e99`), the distance-fade alpha-window numerator `k`.
pub const FADE_K_F32_BITS: u32 = 0x3fea_6e99;
/// `0x8029d4` = `2.384e-07` (`0x34800000`), the back-off / fade-ratio trigger ε.
pub const EPS_BACK_F32_BITS: u32 = 0x3480_0000;

#[inline]
fn ms_to_s_f32() -> f64 {
    f64::from(f32::from_bits(MS_TO_S_F32_BITS))
}
#[inline]
fn one_f32() -> f64 {
    f64::from(f32::from_bits(ONE_F32_BITS))
}
#[inline]
fn half_84f2d4() -> f64 {
    f64::from(f32::from_bits(HALF_84F2D4_F32_BITS))
}
#[inline]
fn half_7ffa24() -> f64 {
    f64::from(f32::from_bits(HALF_7FFA24_F32_BITS))
}
#[inline]
fn quarter_f32() -> f64 {
    f64::from(f32::from_bits(QUARTER_F32_BITS))
}
#[inline]
fn mood_fade_f32() -> f64 {
    f64::from(f32::from_bits(MOOD_FADE_F32_BITS))
}
#[inline]
fn deg2rad_f32() -> f64 {
    f64::from(f32::from_bits(DEG2RAD_F32_BITS))
}
#[inline]
fn ninth_f32() -> f64 {
    f64::from(f32::from_bits(NINTH_F32_BITS))
}
#[inline]
fn recenter_bias_f32() -> f64 {
    f64::from(f32::from_bits(RECENTER_BIAS_F32_BITS))
}
#[inline]
fn fade_a_f32() -> f64 {
    f64::from(f32::from_bits(FADE_A_F32_BITS))
}
#[inline]
fn fade_b_f32() -> f64 {
    f64::from(f32::from_bits(FADE_B_F32_BITS))
}
#[inline]
fn fade_k_f32() -> f64 {
    f64::from(f32::from_bits(FADE_K_F32_BITS))
}

use benilla_re_cmath::x87::{fistp, ftol};

// ===========================================================================================
// (1) 0x4fda20 GetTradeSkillCooldown — `fild qword [ebp-0x14]; fmul dword [0x801360]=1/1000(f32);
//     fstp qword [esp]` @ 0x4fdaad..0x4fdabe (true_ret 0x4fdadd).
// ===========================================================================================

/// `0x4fda20`'s owned arithmetic (`fmul dword [0x801360]` @ `0x4fdab5`): the cooldown remainder
/// `(start + dur) - GetTickCount` is written to `[ebp-0x14]` with `[ebp-0x10]` forced to `0`, so the
/// `fild qword` reads it as a **zero-extended 64-bit** value (the decomp's `(uint)…` cast — a
/// negative i32 reads as a large positive u32). It is `·(1/1000)` (the f32 at `0x801360`, widened)
/// and `fstp`'d as a **qword** (the un-narrowed `f64`) straight into the `lua_pushnumber` double arg.
/// `remainder` is the 32-bit `(start+dur)-tick` value (interpreted unsigned). Returns the f64 seconds.
pub fn tradeskill_cooldown_seconds(remainder: i32) -> f64 {
    // fild qword with hi-dword 0 == convert the value zero-extended to 64-bit (unsigned 32-bit).
    f64::from(remainder as u32) * ms_to_s_f32()
}

// ===========================================================================================
// (2) 0x507710 — status-bar %…-format text builder. Owned FP islands:
// ===========================================================================================

/// `0x507710` `/n` reciprocal scale (`fild dword [ebp-0xc]; fdivr dword [0x7ff9d8]=1.0; fstp [ebp-0x8]`
/// @ `0x50777c`..`0x50778a`). When the format begins `'/'<digits>`, `local_c = 1.0 / n` where `n` is
/// the parsed integer; `fdivr [1.0]` SWAPS operands so it is `1.0 / st0`, not `st0 / 1.0`. (For `'*'`
/// the `fdivr` is skipped and `local_c = (float)n` directly — that path has no FP-arith.) `n` is the
/// parsed integer scale. Returns `local_c` as the stored f32.
pub fn recip_scale(n: i32) -> f32 {
    (one_f32() / f64::from(n)) as f32 // fild dword [n]; fdivr [1.0]; fstp dword
}

/// `0x507710` `M/O/S/m/o/s` value pre-scale (`fild [ebp+0x10]; fld [ebp+0xc]; fmul st,st(1);
/// fstp [ebp+0xc]; fmul [ebp+0x18]; fstp [ebp+0x18]` @ `0x5078d1`..`0x5078e4`): `param_4 *= (int)p5;
/// param_7 = (int)p5 * param_7`. The integer `(int)p5` (`fild`) stays LIVE `f64` on `st(1)` across
/// BOTH multiplies (it is never spilled); `param_4`/`param_7` enter as `f32` memory operands. So
/// `out_p4 = (f32)(param_4·ip5)`, `out_p7 = (f32)(ip5·param_7)` (multiply commutes; the LIVE-f64
/// multiplicand is the same value for both). Returns `(param_4·ip5, ip5·param_7)` as stored f32s.
pub fn mos_prescale(param_4: f32, param_7: f32, p5_int: i32) -> (f32, f32) {
    let ip5 = f64::from(p5_int); // fild dword [ebp+0x10] (exact), LIVE on st(1)
    let out_p4 = (f64::from(param_4) * ip5) as f32; // fld [ebp+0xc]; fmul st,st(1); fstp dword
    let out_p7 = (ip5 * f64::from(param_7)) as f32; // fmul [ebp+0x18]; fstp dword
    (out_p4, out_p7)
}

/// `0x507710` the `'o'/'O'` per-tick divide (`fild [ebp+0x14]=ticks; fild [ebp+0x8]=denom;
/// fld st(1); fmul [ebp+0xc]; fdiv st,st(1); fstp [ebp+0xc]; fxch st(1); fmul [ebp+0x18]; fdiv st,st(1);
/// fstp [ebp+0x18]; fstp st(0)` @ `0x507926`..`0x507940`): `param_4 = (ticks·param_4)/denom;
/// param_7 = (ticks·param_7)/denom`. `ticks` (`fild`) is LIVE on st(1), `denom` (`fild`) LIVE on
/// st(0) initially then used as the `fdiv st,st(1)` divisor — both stay un-narrowed `f64`; `param_4`/
/// `param_7` are `f32` memory operands. Returns `(param_4', param_7')` as stored f32s.
pub fn ot_per_tick(param_4: f32, param_7: f32, ticks: i32, denom: i32) -> (f32, f32) {
    let t = f64::from(ticks); // fild dword [ebp+0x14], LIVE
    let d = f64::from(denom); // fild dword [ebp+0x8], LIVE (the fdiv divisor)
                              // fld st(1)=t ; fmul [ebp+0xc]=param_4 -> t*param_4 ; fdiv st,st(1)=/denom
    let out_p4 = (t * f64::from(param_4) / d) as f32;
    // fxch ; fmul [ebp+0x18]=param_7 -> t*param_7 ; fdiv st,st(1)=/denom
    let out_p7 = (t * f64::from(param_7) / d) as f32;
    (out_p4, out_p7)
}

/// `0x507710` one half of the minutes/seconds **round-to-nearest** island
/// (`fld v; fabs; fmul [ebp-0x8]=local_c; fst [ebp+0x8]; ... fadd st,st; fsub/fadd [0.5]; fistp; sar`).
/// For each of `param_4` (→ `fVar10`) and `param_7` (→ `param_6`): `m = |v|·local_c`, narrowed to f32
/// at the `fst`/`fstp` slot, then `r = (i32)round_via_x87(2·m_f32 ± 0.5) >> 1` where the sign of
/// the `0.5` is `+` when `m < 0.0` (the LIVE f64 `m` decides the branch) and `−` when `m ≥ 0.0`. The
/// `fadd st,st` doubles the **reloaded f32** `m`; the `±0.5` then `fistp` rounds via the current x87
/// mode (PC_53, round-to-nearest-even), and `sar` halves. Since `m = |v|·local_c` enters ABS, the
/// `m < 0.0` branch is only reachable when `local_c < 0` — preserved op-for-op anyway.
///
/// Returns `(m_f32, half)` where `m_f32` is the f32-narrowed magnitude (the binary's `[ebp+0x8]`/
/// `[ebp+0x14]` slot, reused downstream by the floor/ceil snap) and `half = (round(2·m_f32 ± 0.5))>>1`.
pub fn mos_round_half(v: f32, local_c: f32) -> (f32, i32) {
    let m_live = f64::from(v).abs() * f64::from(local_c); // fld; fabs; fmul [local_c]  (LIVE f64)
    let m_f32 = m_live as f32; // fst/fstp dword [slot]  (narrowed)
    let doubled = f64::from(m_f32) + f64::from(m_f32); // fld [slot]; fadd st,st  (reloaded f32, ×2)
                                                       // branch decided by the LIVE f64 m vs 0.0:
    let biased = if m_live < 0.0 {
        doubled + half_84f2d4() // fadd dword [0.5]
    } else {
        doubled - half_84f2d4() // fsub dword [0.5]
    };
    // fistp dword [..] : x87 round-to-nearest-even of the PC_53 value, then arithmetic `sar` halves.
    let rounded = round_nearest_even_i32(biased);
    (m_f32, rounded >> 1)
}

/// x87 `fistp` to a 32-bit int under the default rounding mode (round-to-nearest, ties-to-even).
#[inline]
fn round_nearest_even_i32(v: f64) -> i32 {
    fistp(v)
}

/// `0x507710` the `'f'`-case scale (`fld [ebp-0x8]=local_c; fmul dword [esi+4*edi+0x29c]; fstp qword`
/// @ `0x507ff7`..`0x508007`): `local_c · field` where `field` is the spell-effect float at
/// `[base + 0x29c + idx*4]`, kept as `f64` (pushed as a `lua_pushnumber` double arg). `local_c` and
/// `field` are `f32` memory operands widened to f64. Returns the f64 product.
pub fn f_case_scale(local_c: f32, field: f32) -> f64 {
    f64::from(local_c) * f64::from(field) // fld [local_c]; fmul dword [field]; fstp qword
}

/// `0x507710` the `'f'`/`'a'`-alt quantize (`fld [ebp-0x8]; fmul [esi+4*edi+0x29c]; fcom [0x7ffd74]=0;
/// fadd/fsub [0x7ffa24]=0.5; __ftol` @ `0x508025`..`0x50804a`): `q = local_c·field`, then
/// `(i32)trunc(q + 0.5)` if `q > 0` else `(i32)trunc(q − 0.5)` (the `fcom` vs 0.0 picks the sign of
/// the round bias; `__ftol` truncates toward zero). `local_c`,`field` are f32 widened. Returns the
/// quantized i32.
///
/// The `fcom [0]; fnstsw; test ah,0x41; jne` reads C0|C3: `jne` (ZF) is taken when the masked AH is
/// non-zero, i.e. when `q < 0` (C0) **or** `q == 0` (C3) — both take the `fsub [0.5]` branch; only
/// `q > 0` (C0=C3=0, AH&0x41==0) falls through to `fadd [0.5]`.
pub fn f_case_quantize(local_c: f32, field: f32) -> i32 {
    let q = f64::from(local_c) * f64::from(field); // fld; fmul dword
    let biased = if q > 0.0 {
        q + half_7ffa24() // fadd [0.5]  (only when q strictly > 0)
    } else {
        q - half_7ffa24() // fsub [0.5]  (q < 0 OR q == 0)
    };
    ftol(biased) // __ftol truncate toward zero
}

// ===========================================================================================
// (3) 0x508680 — "garbled chat" filter. Owned FP @ [0x508680, 0x508a53):
//   3x `fld [ebp-4]; fsub dword [0x7ff9d8]=1.0; fcomp [ebp+0xc]=param_4`  (the noise-vs-threshold test)
//   1x `... fld [ebp+0xc]; fmul dword [0x8029b0]=0.25; fcompp`           (the param_4·0.25 scale)
// ===========================================================================================

/// `0x508680`'s noise-bias subtract (`fld dword [ebp-0x4]=local_8; fsub dword [0x7ff9d8]=1.0` @
/// `0x50882f` / `0x508993` / `0x5089ee`, byte-identical): `local_8` is a hashed-noise float built as
/// `(rand & 0x7fffff | 0x3f800000)` — an IEEE float in `[1.0, 2.0)` — and `local_8 − 1.0` brings it
/// to `[0.0, 1.0)`. The result is compared (`fcomp`/`fcompp`, a compare — excluded) against `param_4`
/// (or `param_4·0.25`). `local_8` is the `f32` noise value (widened). Returns `local_8 − 1.0` (the
/// value the source `fsub` produces on st0 before the compare).
pub fn garble_noise_minus_one(local_8: f32) -> f64 {
    f64::from(local_8) - one_f32() // fld dword [local_8]; fsub dword [1.0]  (PC_53 f64)
}

/// `0x508680`'s threshold scale (`fld dword [ebp+0xc]=param_4; fmul dword [0x8029b0]=0.25` @
/// `0x5089f7`, in the end-of-string flush case): `param_4 · 0.25` — the noise gate for the trailing
/// flush is a quarter of the per-char gate. `param_4` is the `f32` threshold (widened). Returns the
/// f64 product (compared via `fcompp` against `local_8 − 1.0`).
pub fn garble_threshold_quarter(param_4: f32) -> f64 {
    f64::from(param_4) * quarter_f32() // fld dword [param_4]; fmul dword [0.25]
}

// ===========================================================================================
// (4) 0x50e8d0 — the camera driver. Owned fade/rate islands @ [0x50e8d0, 0x50f047):
// ===========================================================================================

/// `0x50e8d0` mood-fade enable test (`fild [ebp-0x14]=clamped byte; fmul dword [0x8029d0]=0.01;
/// fcomp [0x7ffd74]=0` @ `0x50eb93`..`0x50eb9c`): a creature "mood" byte (read from `[..+0x1d]`,
/// clamped to `≤ 100`) is `·0.01` and compared `!= 0.0` to decide whether the mood-fade path runs.
/// `byte` is the clamped mood byte (`0..=100`). Returns `byte·0.01` as the f64 product (the value the
/// source `fmul` leaves on st0 before the `fcomp`).
pub fn mood_fade(byte: u8) -> f64 {
    f64::from(i32::from(byte)) * mood_fade_f32() // fild dword; fmul dword [0.01]
}

/// `0x50e8d0` the two look-rate degrees→radians scales (`fld dword [eax+0x24]; fmul dword [0x7ffaac]=
/// π/180; fstp dword` @ `0x50ec18`/`0x50ec24`/`0x50ecf5`): a per-frame angular rate field (`[..+0x24]`)
/// is `·(π/180)` to radians. `rate_deg` is the rate field as f32 (widened). Returns the radians as the
/// stored f32 (`[ebp-0x1c]`/`[ebp-0x18]`/`[ebp+0xc]`).
pub fn look_rate_rad(rate_deg: f32) -> f32 {
    (f64::from(rate_deg) * deg2rad_f32()) as f32 // fld dword; fmul dword [π/180]; fstp dword
}

/// `0x50e8d0` the post-resolve pitch accumulate (`fld [ebp-0x30]=local_34; fadd dword [ebp-0x4]=local_8;
/// fstp [ebp-0x30]` @ `0x50ed5e`..`0x50ed6e`): `local_34 = local_34 + local_8` where `local_8 =
/// local_14 + param_1[0x41]` (the resolved pitch delta). Both `f32` widened; result stored f32.
/// Returns `local_34 + local_8` as the stored f32.
pub fn pitch_accumulate(local_34: f32, local_8: f32) -> f32 {
    (f64::from(local_34) + f64::from(local_8)) as f32 // fld dword; fadd dword; fstp dword
}

/// `0x50e8d0` the `local_8 = local_14 + param_1[0x41]` pre-accumulate
/// (`fld [ebp-0x10]=local_14; fadd dword [esi+0x104]; fstp [ebp-0x4]` @ `0x50eb52`..`0x50eb5f`):
/// the resolved zoom/pitch component plus a camera state field. Both f32 widened; result stored f32.
pub fn local8_preadd(local_14: f32, field_104: f32) -> f32 {
    (f64::from(local_14) + f64::from(field_104)) as f32 // fld dword; fadd dword; fstp dword
}

/// `0x50e8d0` the `|field| vs ε` magnitude tests (`fld dword [m]; fabs; fcomp [ε]`): used for
/// `|local_1c| ≥ 2.384e-7` (`0x50ebdb`, ε @ `0x8029d4`) and `|param_1[0x41]| ≥ 1e-3` (`0x50ee38`,
/// ε @ `0x801360`). The `fabs` is the owned FP-arith (the `fcomp` is a compare). `v` is the f32 field
/// (widened). Returns `|v|` (the value the source `fabs` leaves on st0 before the compare).
pub fn field_abs(v: f32) -> f64 {
    f64::from(v).abs() // fld dword; fabs  (PC_53 f64)
}

/// `0x50e8d0` the per-axis overshoot-recenter (`fld [esi+0x..]=target; fsub [ebp+0xc]=cur;
/// fcomp [0x808a..]=1/9; fld [ebp+0xc or -0x4]; fadd dword [1/9]; fadd dword [0x8026bc]=bias; fst/fstp`
/// @ `0x50ee5d`..`0x50eeaa` (yaw/dist axis) and `0x50eeb0`..`0x50eee6` (pitch axis)). When the
/// target-minus-current distance exceeds `1/9`, the target is recentered to `cur + 1/9 + bias`
/// (a tiny `9.5367e-7` positive bias so the next frame reads it as strictly-positive). The recentered
/// value is stored to TWO fields via `fst` (NO pop, st0 LIVE) then `fstp` — both stores write the
/// SAME `f32` (the live-st0 value equals the f32 it was just `fst`'d to, so no live-vs-reload split
/// here). `cur` is `param_3` (the yaw axis) or `local_8`/`[ebp-0x4]` (the pitch axis); the recenter
/// adds the `1/9` const then the `bias`. Returns `(over: bool, recentered_f32)` — `over` is
/// `target − cur > 1/9`, and when `over`, `recentered = (cur + 1/9 + bias)` as the stored f32; else
/// `recentered = cur` unchanged.
pub fn overshoot_recenter(target: f32, cur: f32) -> (bool, f32) {
    // fld [target]; fsub [cur]; fcomp [1/9]  ->  (target - cur) compared to 1/9.
    // The source `test ah,0x41; jne` (C0|C3) takes the recenter branch only when AH&0x41==0, i.e.
    // `target - cur > 1/9` strictly (C0=0 not-below, C3=0 not-equal).
    let diff = f64::from(target) - f64::from(cur); // fld; fsub  (owned)
    let frac = ninth_f32();
    let over = diff > frac; // fcomp [1/9]; the C0|C3 test => strict >
    if over {
        // fld [cur]; fadd [1/9]; fadd [bias]; fst/fstp dword  (the recenter, narrowed to f32)
        let recentered = ((f64::from(cur) + frac) + recenter_bias_f32()) as f32;
        (true, recentered)
    } else {
        (false, cur)
    }
}

/// `0x50e8d0` the distance-fade alpha window. Two sub-parts diffed together because the binary leaves
/// the ratio LIVE on st0 across the `fabs` test (`fst`-no-pop trap):
///
/// 1. ratio (`fld [0x808a94]=A; fsub [esi+0xf4]=d; fld [0x808a94]=A; fsub [0x808a98]=B; fdivp st(1),st;
///    fst [ebp-0x4]; fabs; fcomp [0x8029d4]=ε` @ `0x50ef27`..`0x50ef46`): `r = (A − d) / (A − B)`,
///    stored to `[ebp-0x4]` as f32 (NO pop) while the LIVE f64 `r` is `fabs`'d and tested `≥ ε`.
/// 2. window (`fld [ebp-0x4]; fmul [ebp-0x4]; fdivr [0x8089b0]=k` @ `0x50ef55`..`0x50ef5b`): when the
///    test passes, `fade = k / (r_f32 · r_f32)` — the square reloads the **f32** `r` (the narrowed
///    `[ebp-0x4]`), NOT the live f64; `fdivr [k]` SWAPS to `k / (r·r)`.
///
/// `d` is the camera distance field `param_1[0x3d]` (f32). Returns `(r_f32, fade_opt)` where `r_f32`
/// is the f32-narrowed ratio and `fade_opt = Some(k/(r_f32·r_f32))` when `|r_live| ≥ ε`, else `None`
/// (the binary keeps the default `fVar14 = k` — represented here as `None`).
pub fn distance_fade(d: f32) -> (f32, Option<f32>) {
    let a = fade_a_f32();
    let b = fade_b_f32();
    let r_live = (a - f64::from(d)) / (a - b); // fld[A];fsub[d]; fld[A];fsub[B]; fdivp  (LIVE f64)
    let r_f32 = r_live as f32; // fst [ebp-0x4]  (narrowed; st0 stays live)
    let eps = f64::from(f32::from_bits(EPS_BACK_F32_BITS)); // 2.384e-7 @ 0x8029d4
                                                            // fabs on the LIVE f64 r; fcomp [ε]; `test ah,5; jnp` (C0|C2) => skip when |r| < ε;
                                                            // falls through to the square when |r| >= ε.
    if r_live.abs() >= eps {
        let sq = f64::from(r_f32) * f64::from(r_f32); // fld [ebp-0x4]; fmul [ebp-0x4]  (RELOADED f32)
        let fade = (fade_k_f32() / sq) as f32; // fdivr [k]  => k / (r_f32·r_f32)
        (r_f32, Some(fade))
    } else {
        (r_f32, None)
    }
}

/// `0x50e8d0` the fade-distance subtract + alpha divide
/// (`fld [ebp+0xc]=param_3; fsub [esi+0x38]=field; fst [ebp+0xc]; fcomp st(1)` @ `0x50ef61`..`0x50ef6a`,
/// then in the `param_3 < fVar14 && param_3 > 0x808abc` case `fld [ebp+0xc]; fdiv st,st(1)` @
/// `0x50ef96`..`0x50ef9a`): `param_3 = param_3 − field`, then `alpha = param_3 / fVar14` (passed to
/// `0x5b7bb0`·`__ftol` to a 0..=255 byte). The `fsub` and `fdiv` are owned; `param_3`,`field` are
/// f32 widened, `fVar14` is the live fade value. `param_3` is the camera fade-distance, `field` the
/// `param_1[0xe]` near-plane field, `fade` the `fVar14` window. Returns `(param_3', ratio)` where
/// `param_3' = (param_3 − field)` stored f32 and `ratio = param_3' / fade` as f64 (the `fdiv` result).
pub fn fade_alpha_ratio(param_3: f32, field: f32, fade: f32) -> (f32, f64) {
    let p3 = (f64::from(param_3) - f64::from(field)) as f32; // fld; fsub [field]; fst dword (narrowed)
    let ratio = f64::from(p3) / f64::from(fade); // fld [ebp+0xc] (reloaded f32); fdiv st,st(1)
    (p3, ratio)
}

/// `0x50e8d0` the camera-shake `fsincos` (`fld dword [ebp+0xc]=angle; fsincos; fstp dword [eax];
/// fstp dword [edx]` @ `0x50efe5`..`0x50efec`): when the camera is in the close-shake band, the shake
/// angle (`local_10`, the resolved roll) is run through `fsincos`, storing `cos` then `sin` as f32.
/// **Transcendental → libm**: diffed structurally (the emulator routes `fsincos`
/// through the host libm). `angle` is the f32 shake angle (widened). Returns `(cos, sin)` as stored
/// f32s (the `fstp [eax]`=cos popped first, `fstp [edx]`=sin).
pub fn shake_sincos(angle: f32) -> (f32, f32) {
    let (sin, cos) = f64::from(angle).sin_cos(); // fsincos: st0->cos (popped first), st1->sin
    (cos as f32, sin as f32)
}

// ===========================================================================================
// (5) 0x507710 the M/O/S floor/ceil integer-snap correction (`0x5079cc`..`0x507ac3`), one per axis.
// Covers the 6 owned `fsubr/fsub [slot]` ops following the CRT floor (`0x73ff3f`) / ceil
// (`0x73fdf5`) calls (verified empirically in the harness: 0x73ff3f=floor, 0x73fdf5=ceil — the
// decomp's helper labels). The ceil/floor are CRT (DELEGATED, run natively in the diff). NB the
// `fsubr [slot]` SWAPS (`[slot] − st0` = `m − floor(m)`); the `fsub [slot]` is `ceil(m) − m`.
// ===========================================================================================

/// One axis of `0x507710`'s integer-snap correction, following [`mos_round_half`]. Given the f32
/// magnitude `m_f32` (the `[ebp+0x8]`/`[ebp+0x14]` slot) and the round-half `half` (= `(round(2·m
/// ±0.5))>>1`), the binary tests near-integrality and may bump `half` by 1:
///
/// ```text
///   is_int = (m − floor(m) < 1e-3)            // 0x5079df fsubr [slot] ([slot]−st0); fcomp [1e-3]; jnp
///         || (ceil(m) − m < 1e-3)             // 0x507a00 fsub [slot] (st0−[slot]); fcomp [1e-3]; jnp
///   if is_int {
///     bVar = true
///     if half != (i32)ftol(ceil(m))          // 0x507a25 ceil; 0x507a2d __ftol; cmp esi,eax; je
///        && (ceil(m) − m < 1e-3)              // 0x507a3f ceil; 0x507a44 fsub [slot]; fcomp; jp
///     { half += 1 }
///   } else { bVar = false }
/// ```
///
/// `m_f32` is the slot magnitude; `half` the round-half result. The `ftol` truncates toward zero.
/// Returns `(bVar, corrected_half)` — `bVar` is the `bVar1`/`bVar2` "near-integer" flag the M/O/S
/// dispatch reads, `corrected_half` the (possibly +1'd) half value the binary stores back.
pub fn mos_snap(m_f32: f32, half: i32) -> (bool, i32) {
    let m = f64::from(m_f32);
    let eps = ms_to_s_f32(); // 1e-3 @ 0x801360
    let floor_m = m_f32.floor(); // 0x73ff3f = floor
    let ceil_m = m_f32.ceil(); // 0x73fdf5 = ceil
    let frac_gap = m - f64::from(floor_m); // fsubr [slot]: [slot]-floor = m - floor(m)
    let ceil_gap = f64::from(ceil_m) - m; // fsub [slot]: ceil - [slot] = ceil(m) - m
    let is_int = frac_gap < eps || ceil_gap < eps;
    let mut corrected = half;
    if is_int {
        let ceil_ftol = ftol(f64::from(ceil_m)); // __ftol(ceil(m)) truncate
        if half != ceil_ftol && ceil_gap < eps {
            corrected = half + 1;
        }
    }
    (is_int, corrected)
}

// ===========================================================================================
// (6) 0x50e8d0 — the owned camera-position vector composition (the `local_8==0 && uVar11==0` branch,
// `0x50ea8d`..`0x50eafc`). Composes the eye-offset vector from the camera distance scalar
// `fVar1 = p1[0x3b]` and a DELEGATED view direction vector `pfVar9` (from `(**[*p1+4])(...)`). The
// view vector is delegated, but the scalar-times-vector + per-axis subtract IS this driver's own
// arithmetic, diffed bit-exact. (`breaks toward PRIMITIVE`: in-body owned FP, not behind a call.)
// ===========================================================================================

/// `0x50e8d0` the height pre-add (`fld [ebp-0x30]=local_34; fadd dword [esi+0xfc]=p1[0x3f]; fstp
/// [ebp-0x54]` @ `0x50ea8d`..`0x50eaa4`): `local_58 = local_34 + p1[0x3f]` (the eye Z plus a camera
/// height field). Both f32 widened; result stored f32. Returns the sum as the stored f32.
pub fn view_height_add(local_34: f32, field_fc: f32) -> f32 {
    (f64::from(local_34) + f64::from(field_fc)) as f32 // fld; fadd dword; fstp dword
}

/// `0x50e8d0` the eye-offset compose (`fld [esi+0xec]=fVar1; fld st; fmul [eax]; fld st(1);
/// fmul [eax+4]; fstp [ebp-0x58]; fxch; fmul [eax+8]; fstp [ebp-0x54]; fld [ebp-0x50]; fsub st,st(1);
/// fstp [ebp-0x50]; fstp st(0); fld [ebp-0x4c]; fsub [ebp-0x58]; fstp [ebp-0x4c]; fld [ebp-0x48];
/// fsub [ebp-0x54]; fstp [ebp-0x48]` @ `0x50eabc`..`0x50eafc`). `fVar1 = p1[0x3b]` (the camera
/// distance scalar) stays LIVE f64 on the stack across all three `fmul`s with the DELEGATED view
/// vector `v=(v0,v1,v2)`; then each base component is subtracted by the scaled view:
///
/// ```text
///   p1 [ebp-0x58] = fVar1·v1       (stored f32)
///   p2 [ebp-0x54] = fVar1·v2       (stored f32)
///   base_x [ebp-0x50] −= fVar1·v0  (live f64 fVar1·v0; fsub st,st(1))
///   base_y [ebp-0x4c] −= p1        (reloaded f32 p1)
///   base_z [ebp-0x48] −= p2        (reloaded f32 p2)
/// ```
///
/// NB the x-axis subtract uses the LIVE f64 `fVar1·v0` (never spilled — live-vs-reload edge),
/// while y/z subtract the f32-narrowed `p1`/`p2`. `fvar1` is the distance scalar; `v` the view
/// vector; `base` the (x,y,z) accumulator. Returns the five stored f32s
/// `(p1=fVar1·v1, p2=fVar1·v2, out_x, out_y, out_z)`.
pub fn view_eye_offset(fvar1: f32, v: [f32; 3], base: [f32; 3]) -> (f32, f32, f32, f32, f32) {
    let f = f64::from(fvar1); // fld [esi+0xec], LIVE across all fmuls
    let prod0_live = f * f64::from(v[0]); // fmul [eax]    -> stays LIVE f64
    let p1 = (f * f64::from(v[1])) as f32; // fmul [eax+4]; fstp [ebp-0x58]  (narrowed)
    let p2 = (f * f64::from(v[2])) as f32; // fmul [eax+8]; fstp [ebp-0x54]  (narrowed)
    let out_x = (f64::from(base[0]) - prod0_live) as f32; // fsub st,st(1) (LIVE f64); fstp
    let out_y = (f64::from(base[1]) - f64::from(p1)) as f32; // fsub [ebp-0x58] (reloaded f32); fstp
    let out_z = (f64::from(base[2]) - f64::from(p2)) as f32; // fsub [ebp-0x54] (reloaded f32); fstp
    (p1, p2, out_x, out_y, out_z)
}

/// `0x50e8d0` the eye-offset accumulate (`fld [ebp-0x44]; fadd dword [edi]; fstp [ebp-0x44]` ×3 @
/// `0x50eb0f`..`0x50eb3b`): the three offset components `local_48/44/40` each `+= p1[0x44..0x46]`
/// (`edi = p1+0x110`, so `[edi]`=`p1[0x44]`, `[edi+4]`=`p1[0x45]`, `[edi+8]`=`p1[0x46]`). Each f32
/// widened; results stored f32. `acc` is the three accumulator components, `field` the `p1[0x44..]`
/// vector. Returns the three sums as stored f32s.
pub fn view_offset_accum(acc: [f32; 3], field: [f32; 3]) -> (f32, f32, f32) {
    (
        (f64::from(acc[0]) + f64::from(field[0])) as f32, // fld [ebp-0x44]; fadd [edi];   fstp
        (f64::from(acc[1]) + f64::from(field[1])) as f32, // fld [ebp-0x40]; fadd [edi+4]; fstp
        (f64::from(acc[2]) + f64::from(field[2])) as f32, // fld [ebp-0x3c]; fadd [edi+8]; fstp
    )
}

/// `0x507710` mos_snap ceil-fraction — the one snap op (`0x507af0 fsub dword [ebp+0x14]`) that sits
/// in the ceil-double-check block past the p4/p7 snap slices' halt (reached when `edi != ftol(ceil(m))`).
/// After the CRT `ceil` (`0x73fdf5`) leaves `ceil(x)` on st0 (its f64 arg is the f32 `x` at `[ebp+0x14]`
/// widened), `fsub dword [ebp+0x14]` subtracts the SAME f32 `x` (widened again) — the snap residual
/// `ceil(x) − x` compared to ε at `0x507af6`. Both reads of `x` widen exactly to f64, and IEEE `ceil`
/// is exact, so the result is `ceil(f64::from(x)) − f64::from(x)`. The difftest runs the binary's REAL
/// `ceil` then this `fsub` (verbatim-byte island), so the stub itself settles the ceil match.
pub fn ceil_frac(x: f32) -> f64 {
    let xf = f64::from(x);
    xf.ceil() - xf // fld [ebp+0x14] (push x f64); call ceil -> st0=ceil(x); fsub dword [ebp+0x14]
}
