//! Reimplements the arithmetic of the camera shake and transition functions from
//! `CCamera::BeginBobDecay` (`0x511110`) to `CCamera::BeginHeightTransition` (`0x512790`): the
//! drunk field of view, shakes, distance and height transitions. The x87 runs at 53-bit
//! precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in
//! `tests/camera_shake_transitions.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

/// dt = 0.001 (`DAT_00801360`), the change-detect tolerance.
pub const DT_BITS: u32 = 0x3a83_126f; // 0.0010000000474974513
/// norm-ε = 2⁻²² (`DAT_008029d4`), the eps change-detect.
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07
/// 1.0 (`DAT_007ff9d8`).
pub const ONE_BITS: u32 = 0x3f80_0000;
/// 0.0 (`DAT_007ffd74`).
pub const ZERO_BITS: u32 = 0x0000_0000;
/// `DAT_008089b8` = 3.1241393089 — the lead constant of `0x511250`'s `(C − π/2)·arg` scale.
pub const C_8089B8_BITS: u32 = 0x4047_f1e6; // 3.1241393089294434
/// `DAT_008089b4` = π/2 = 1.5707963705 — the subtrahend of `0x511250`.
pub const HALF_PI_BITS: u32 = 0x3fc9_0fdb; // 1.5707963705062866
/// `DAT_007ff9d0` = 1/36 = 0.0277778 — the scale of `0x511d40`.
pub const RECIP36_BITS: u32 = 0x3ce3_8e39; // 0.02777777798473835
/// `DAT_00801448` = −1000.0 — the `fmul` of `0x512120`/`0x5126b0`'s `__ftol` term.
pub const NEG1000_BITS: u32 = 0xc47a_0000; // -1000.0

#[inline]
fn dt() -> f64 {
    f64::from(f32::from_bits(DT_BITS))
}
#[inline]
fn eps() -> f64 {
    f64::from(f32::from_bits(EPS_BITS))
}

// ===========================================================================================
// (1) 0x511110 — |field| / divisor  (fabs; fdiv)
// ===========================================================================================

/// `0x511110` (`0x511132`..`0x511162`): `fld [esi+4*eax+0x110]; fabs; fdiv [div+0x24]; fstp
/// [esi+0x23c]`. The selected channel field (`eax` = the channel index from the delegated
/// `0x7bf680` call) is made positive and divided by the per-frame divisor `*([0xbe10bc])+0x24`,
/// stored as f32 (`field` = the channel value, `divisor` = `[div+0x24]`). 2 owned FP ops.
pub fn abs_div(field: f32, divisor: f32) -> f32 {
    (f64::from(field).abs() / f64::from(divisor)) as f32
}

// ===========================================================================================
// (2) 0x511250 — (C − π/2)·arg scale + eps change-detect  (fsub; fmul / fsub; fabs)
// ===========================================================================================

/// `0x511250` scale (`0x511253`..`0x51126a`): `fld [0x8089b8=3.1241393]; fsub [0x8089b4=π/2]; fmul
/// [ebp+0x8]=arg; fstp [ebp+0x8]`. The constant difference `3.1241393 − π/2 = 1.5533429` is the
/// 89° drunk-FOV span (`179° − 90°`; NOT the ±89° pitch-clamp constant `0x8089d8=1.5533430576` —
/// distinct bits): `0x511250` is the drunk camera-FOV setter, arg = the drunk fraction, target =
/// `89°·arg` on channel `+0x10c`. 2 owned FP ops
/// (fsub of two constants, then fmul by arg).
pub fn scale_const_minus_halfpi(arg: f32) -> f32 {
    let k = f64::from(f32::from_bits(C_8089B8_BITS)) - f64::from(f32::from_bits(HALF_PI_BITS));
    (k * f64::from(arg)) as f32
}

/// `0x511250` change-detect (`0x51126f`..`0x511285`, reached when `[ebp+0xc] != 0`): `fld
/// [esi+0x10c]=field; fsub [ebp+0x8]=scaled; fabs; fcomp [eps=2⁻²²]; fnstsw; test ah,5; jp
/// 0x51128e(=eax=0)`. **PF=1 ⟺ EVEN set-bits**; `jp` (PF=1) taken iff EVEN ⇒ `eax=0`. After
/// `fcomp(|d|, eps)`: C0=1 iff `|d|<eps`. ordered `|d|<eps` → 1 bit (odd) → jp NOT taken → `eax=1`.
/// ordered `|d|≥eps`/`==eps` → even → jp taken → `eax=0`. NaN → C0|C2=2 bits (even) → `eax=0`. The
/// returned `dl` is then `sete(al==0)`, but the owned arithmetic is just `|field − scaled| < eps`
/// (false on NaN). Returns `true` ⇒ `eax=1` ⇒ `|d| < eps`. `scaled` = the value `[ebp+0x8]` left by
/// `scale_const_minus_halfpi`.
pub fn scale_settled(field: f32, scaled: f32) -> bool {
    let d = (f64::from(field) - f64::from(scaled)).abs();
    !d.is_nan() && d < eps()
}

// ===========================================================================================
// (3) 0x511760 — (i64)Δ·dt + field, live-st0 fcomp gate  (fmul; fadd)
// ===========================================================================================

/// `0x511760` rate-accumulate gate (`0x511816`..`0x511828`): `fild qword [ebp-0x20]; fmul
/// [0x801360=dt]; fadd [ebx+0x1c]=field; fst [ebp-0xc]; fcomp [ebx+0x18]=limit`. The low dword of
/// `[ebp-0x20]` is `Δ = [ebp-0x8] − [ebx+0x30]` (the high dword is 0 ⇒ unsigned 32-bit magnitude
/// zero-extended to i64), and `t = Δ·dt + field` is the new channel value. `fst [ebp-0xc]` stores
/// `t` as f32 but leaves st0 LIVE — so the `fcomp [ebx+0x18]` compares the **live f64** `t`, while
/// the later `fld [ebp-0xc]` (0x5118b9) reloads the f32 copy. 2 owned FP ops (fmul, fadd).
/// `0x511816`..`0x511822`: the LIVE st0 value `t = (i64)Δ·dt + field` (PC_53 f64) that st0 holds at
/// the `fcomp` gate — BEFORE the `fst [ebp-0xc]` f32 spill (which stores but does NOT pop). This,
/// not the f32 copy, is what the `fcomp [ebx+0x18]` compares.
pub fn rate_accumulate_live(delta: u32, field: f32) -> f64 {
    (i64::from(delta) as f64) * dt() + f64::from(field)
}

pub fn rate_accumulate(delta: u32, field: f32) -> f32 {
    rate_accumulate_live(delta, field) as f32
}

/// The f32 value `fst [ebp-0xc]` stores (the reloaded copy handed downstream at `0x5118b9`).
pub fn rate_accumulate_f32(t: f32) -> f32 {
    t
}

/// `0x511760` live-st0 gate (`0x511828`..`0x51182d`): `fcomp [ebx+0x18]=limit; fnstsw; test ah,1;
/// jne 0x51186a`. `test ah,1` masks C0; `jne` taken (ZF=0 ⇒ masked-AH ≠ 0) iff C0=1 ⇒ the **live
/// f64** `t < limit` (or unordered/NaN — C0 set). `jne` taken → the in-range path (0x51186a, the
/// interp); falls through → the `t ≥ limit` reset path. Returns `true` ⇒ C0 set ⇒ `t < limit`
/// (true on NaN, matching the hardware's unordered C0). **Takes the LIVE f64 `t`** (from
/// `rate_accumulate_live`), NOT the f32 spill: `fst [ebp-0xc]` leaves st0 LIVE, so the `fcomp`
/// reads the un-rounded f64. The divergence bites when `limit == (t as f32)` and `t` rounded UP
/// (live `t < limit`, but the f32 copy `== limit`): live-f64 gate → below, an f32-reload gate →
/// above. (fst-no-pop — the f32-narrowing variant masks the boundary.)
pub fn rate_accumulate_below_limit(t_live: f64, limit: f32) -> bool {
    t_live.is_nan() || f64::from(limit).is_nan() || t_live < f64::from(limit)
}

// ===========================================================================================
// (4) 0x511ad0 — two-stage residual subtract with live-st0 fcom chain  (fsub; fsub)
// ===========================================================================================

/// `0x511ad0` first compare gate (`0x511b5b`..`0x511b6c`): `fld [ebp+0x8]=arg; fsub [ret+0x8]=field8;
/// fcom [esi+0x1c8]=thrA; fnstsw; test ah,5; jnp 0x511ba4`. `d = arg − field8` (where `field8` is
/// `[ret+0x8]` from the delegated vtable `[edx+0x14]` getter), kept LIVE on st0 (`fcom` does NOT
/// pop) so the compare reads the PC_53 f64 `d`, not a reloaded f32. **PF=1 ⟺ EVEN**; `jnp` (PF=0)
/// taken iff ODD ⇒ 0x511ba4 (the `or 0x100000` flag). After `fcom(d, thrA)`: C0=1 iff `d<thrA`.
/// ordered `d<thrA` → 1 bit (odd) → jnp TAKEN. ordered `d≥thrA`/`==`/NaN → even → not taken →
/// continues the second stage. Returns `true` ⇒ ODD ⇒ `d < thrA` (false on NaN) ⇒ the binary takes
/// the `0x511ba4` (set-flag-0x100000) early path. 1 owned FP op (fsub).
pub fn residual_lt_thr_a(arg: f32, field8: f32, thr_a: f32) -> bool {
    let d = f64::from(arg) - f64::from(field8);
    !d.is_nan() && !f64::from(thr_a).is_nan() && d < f64::from(thr_a)
}

/// `0x511ad0` second compare gate (`0x511b6e`..`0x511b81`): reached when `d ≥ thrA` (ordered) —
/// `fld st(0)` duplicates the live `d`, `fsub [esi+0x1c8]=thrA` → `d2 = d − thrA`, `fcom
/// [0x8089d0]=2/9; fnstsw; test ah,5; jnp 0x511ba2`. `d2` is the LIVE PC_53 f64. **PF=1 ⟺ EVEN**;
/// `jnp` (PF=0) taken iff ODD ⇒ `d2 < 2/9`. Returns `true` ⇒ ODD ⇒ `d2 < (2/9)` (false on NaN) ⇒
/// the binary takes the `0x511ba2` (`fstp st(0)` → 0x511ba4) early path. 1 owned FP op (fsub).
/// `d = arg − field8` (the live f64 from the first stage). The final `fcomp [0x8089d4]=5/9` is a
/// pure compare (no arithmetic).
pub fn residual2_lt_2_9(arg: f32, field8: f32, thr_a: f32) -> bool {
    let d = f64::from(arg) - f64::from(field8);
    let d2 = d - f64::from(thr_a);
    let two_ninths = f64::from(f32::from_bits(0x3e63_8e39)); // 0x8089d0 = 0.2222222
    !d2.is_nan() && d2 < two_ninths
}

// ===========================================================================================
// (5) 0x511da0 — negate a delegated getter return  (fchs)
// ===========================================================================================

/// `0x511da0` negated push (`0x511e3a`..`0x511e42`): a delegated vtable getter `[edx+0x18]` returns
/// a value in st0, then `fchs; fstp [esp]` pushes `−x` to `0x5102b0`. The earlier sibling push
/// (`0x511df1`..`0x511df6`) stores the getter return UN-negated (`fstp [ebp+0xc]`, no `fchs`). 1
/// owned FP op (fchs). `x` = the getter return.
pub fn neg_f32(x: f32) -> f32 {
    (-f64::from(x)) as f32
}

// ===========================================================================================
// (5b) 0x511d40 — field · 1/36 scale  (fmul)
// ===========================================================================================

/// `0x511d40` scaled push (`0x511d62`..`0x511d7f`): `fld [eax+0xc]=field; fmul [0x7ff9d0=1/36];
/// fstp [esp]` — the selected table entry's field (`eax` = `[[0xc0dfd0]+4·idx]`, the row pointer)
/// scaled by `1/36 = 0.0277778` and pushed as f32 to the helper `0x511c60`. 1 owned FP op (fmul).
/// `field` = `[eax+0xc]`.
pub fn scale_recip36(field: f32) -> f32 {
    (f64::from(field) * f64::from(f32::from_bits(RECIP36_BITS))) as f32
}

// ===========================================================================================
// (6) 0x512120 / 0x5126b0 — change-detect quartet + body (fsub/fabs ×4 + fsub/fmul/fabs/fdiv/fmul)
//     0x5126b0 is the masked-byte CLONE-TWIN of 0x512120 (offsets/global/rel32 only differ).
// ===========================================================================================

/// `0x512120`/`0x5126b0` change-detect predicate, the shared shape of the four guard tests
/// (`0x512132`/`0x51214a`/`0x512162`/`0x512184` and the twin at `0x5126c2`/…): `fld [field];
/// fsub [arg]; fabs; fcomp [dt=0.001]; fnstsw; test ah,5; jp <out>`. **PF=1 ⟺ EVEN**; `jp` (PF=1)
/// taken iff EVEN ⇒ jumps to the body/out (the "moved past dt" path). After `fcomp(|d|, dt)`:
/// ordered `|d|<dt` → C0=1 (1 bit, odd) → jp NOT taken → continues to the next guard. ordered
/// `|d|≥dt`/`==dt`/NaN → even → jp taken → out. Returns `true` ⇒ ODD ⇒ `|field − arg| < dt`
/// (false on NaN) ⇒ the binary CONTINUES (this axis is settled within dt). 2 owned FP ops (fsub,
/// fabs) per test.
pub fn axis_settled(field: f32, arg: f32) -> bool {
    let d = (f64::from(field) - f64::from(arg)).abs();
    !d.is_nan() && d < dt()
}

/// `0x512120`/`0x5126b0` body (`0x5121a3`..`0x5121f0` / `0x512733`..`0x512780`), reached when all
/// four axes moved ≥ dt. The owned compute, in the binary's exact order:
/// * `fld [ebp+0x8]=arg0; fsub [field_ec]` → `d = arg0 − field_ec` (kept on st, f64) **[fsub]**
/// * `fld [global+0x24]=g` (`global` = `[0xbe106c]`/`[0xbe0cd4]`)
/// * `fld [ebp+0xc]=arg1; fmul [-1000.0]` → `arg1·−1000` **[fmul]**, then `__ftol` truncates it to
///   `eax` (an int used only in integer math: `ecx = arg4 − eax`).
/// * `fxch st(1); fabs` → `|d|` **[fabs]**; `fdiv st,st(1)` → `|d| / g` **[fdiv]**.
/// * `fmul [ebp+0x10]=arg2` → `(|d|/g)·arg2` **[fmul]**, `fstp [esp]` → f32 (the arg pushed to the
///   tail setter `0x512200`/`0x512790`).
///
/// Returns the f32 result `r = (|arg0 − field_ec| / g) · arg2`. 5 owned FP ops in the body.
/// (`g` = the per-frame divisor `*([0xbe106c])+0x24` / `*([0xbe0cd4])+0x24`.)
pub fn body_frac(arg0: f32, field_ec: f32, arg2: f32, g: f32) -> f32 {
    let d = f64::from(arg0) - f64::from(field_ec);
    ((d.abs() / f64::from(g)) * f64::from(arg2)) as f32
}

/// `0x512120`/`0x5126b0` body integer term (`0x5121d0`..`0x5121e1` / `…`): `eax = __ftol(arg1 ·
/// −1000.0)` (truncate toward zero), then `ecx = arg4 − eax`. The owned FP op is the `fmul` (shared
/// with `body_frac`'s recount) feeding `__ftol`; this exposes the resulting integer. `arg1` =
/// `[ebp+0xc]`, `arg4` = `[ebp+0x14]`.
pub fn body_int_term(arg1: f32, arg4: i32) -> i32 {
    let v = f64::from(arg1) * f64::from(f32::from_bits(NEG1000_BITS));
    arg4.wrapping_sub(v.trunc() as i32)
}

// ===========================================================================================
// (7) 0x512200 / 0x512790 — |field − arg| < dt change-detect predicate  (fsub; fabs)
//     Shared OWNED FP slice (the integer tails differ: 0x512790 has an extra sign branch).
// ===========================================================================================

/// `0x512200` (`0x512203`..`0x512216`) / `0x512790` (`0x512793`..`0x5127a6`): `fld [ecx+off]=field;
/// fsub [ebp+0x8]=arg; fabs; fcomp [dt=0.001]; fnstsw; test ah,5; jp <body>`. `off` = `0xec`
/// (`0x512200`) / `0xfc` (`0x512790`). **PF=1 ⟺ EVEN**; `jp` (PF=1) taken iff EVEN ⇒ the body (the
/// commit path). ordered `|d|<dt` → C0=1 (1 bit, odd) → jp NOT taken → `xor eax,eax; ret` (no
/// change). ordered `|d|≥dt`/`==dt`/NaN → even → jp taken → body (commit `eax=1`). Returns `true`
/// ⇒ EVEN ⇒ `|field − arg| ≥ dt` OR unordered (true on NaN) ⇒ the binary COMMITS. 2 owned FP ops
/// (fsub, fabs). Both addresses share this exact FP slice.
pub fn commit_if_moved(field: f32, arg: f32) -> bool {
    let d = (f64::from(field) - f64::from(arg)).abs();
    d.is_nan() || d >= dt()
}
