//! Reimplements the arithmetic of the camera setters `CCamera::SetTarget` (`0x50d0f0`),
//! `SetFovOverride` (`0x50d320`), `LeaveAbsoluteFrame` (`0x50d440`), `PopGroundPitch`
//! (`0x50d520`), `GetFov` (`0x50d6d0`), `ProbeSlope` (`0x50d900`) and `ArmTerrainTilt`
//! (`0x50dbc0`). The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the
//! `fstp dword` stores; the tests in `tests/camera_fov_pitch.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

/// π/180 (`DAT_007ffaac`), deg→rad. (0x50d320 `fmul [0x7ffaac]`.)
pub const DEG2RAD_BITS: u32 = 0x3c8e_fa35; // 0.01745329238474369
/// 0.01 (`DAT_008029d0`), the percent→fraction scale. (0x50d0f0/0x50d1f6 `fmul [0x8029d0]`.)
pub const HUNDREDTH_BITS: u32 = 0x3c23_d70a; // 0.009999999776482582
/// norm-ε = 2⁻²² (`DAT_008029d4`), the change-detect epsilon. (0x50dbc0 `fcomp [0x8029d4]`.)
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07
/// 0.0 (`DAT_007ffd74`). (0x50d6d0 flag source / 0x50dbc0 sign probe.)
pub const ZERO_BITS: u32 = 0x0000_0000;
/// π/2 = 1.5707963705 (`DAT_008089b4`), the 0x50d320 reset value (flag-bit-8 false path).
pub const HALF_PI_BITS: u32 = 0x3fc9_0fdb; // 1.5707963705062866
/// 0.0 (`DAT_008089bc`), the 0x50d6d0 lower clamp bound.
pub const PITCH_LO_BITS: u32 = 0x0000_0000;
/// 3.1241393 (`DAT_008089b8`), the 0x50d6d0 upper clamp bound (≈ π − 0.0174533, i.e. π − 1°).
pub const PITCH_HI_BITS: u32 = 0x4047_f1e6; // 3.1241393089294434

#[inline]
fn deg2rad() -> f64 {
    f64::from(f32::from_bits(DEG2RAD_BITS))
}
#[inline]
fn hundredth() -> f64 {
    f64::from(f32::from_bits(HUNDREDTH_BITS))
}

// ===========================================================================================
// (1) 0x50d320 — deg→rad set / reset-to-π/2 select
// ===========================================================================================

/// `0x50d320` deg→rad convert (`0x50d336`..`0x50d34e`): when the arg `[ebp+8]` is nonzero,
/// `result = (i32)arg · π/180` (`fild [ebp+8]; fmul [0x7ffaac]; fstp [esi+0x40]`) stored f32 to
/// `[esi+0x40]`; the arg is an *integer* count (e.g. a slot index), widened by `fild dword`. The
/// owned op is the single `fmul [π/180]`. (The `arg == 0` path loads the const π/2 and stores it —
/// a const copy, no arithmetic; see [`deg2rad_set_arg`] for the value chosen there.)
pub fn deg2rad_set(arg: i32) -> f32 {
    (f64::from(arg) * deg2rad()) as f32
}

/// The value `0x50d320` stores to `[esi+0x40]` for a given `arg`: `arg != 0` ⇒ `(i32)arg · π/180`
/// (the owned convert); `arg == 0` ⇒ the const π/2 (`fld [0x8089b4]`, stored verbatim — note this
/// path only fires when the `[esi+0x90]` bit-3 flag was previously set, but the stored *value* is
/// the bit-exact π/2 either way). Exposed so the difftest can drive both stores.
pub fn deg2rad_set_arg(arg: i32) -> f32 {
    if arg != 0 {
        deg2rad_set(arg)
    } else {
        f32::from_bits(HALF_PI_BITS)
    }
}

// ===========================================================================================
// (2) 0x50d440 / 0x50d520 — negate-and-push (`fld src; fchs; fstp [esp]`), 5 byte-clone slices
// ===========================================================================================

/// The negate-push kernel shared by every owned-FP slice of `0x50d440` (four `fld src; fchs; fstp
/// [esp]` at 0x50d49f/0x50d4b4/0x50d4d0/0x50d4e5) and `0x50d520` (one at 0x50d528): a source angle
/// is negated (`fchs`) and pushed as an `f32` arg to the angle-channel setter (`0x5102b0` /
/// `0x510120`). `fchs` flips the sign bit (so `-0.0` from a `0.0` source, exactly). The five sites
/// differ only in the source operand (a struct field, a getter return, or the `0.0` literal at
/// `[ebp-4]` in 0x50d4e5) — one kernel covers all by construction.
pub fn neg_push(src: f32) -> f32 {
    (-f64::from(src)) as f32
}

// ===========================================================================================
// (3) 0x50d6d0 — flag-gated source + fadd + clamp to [0.0, 3.1241393]
// ===========================================================================================

/// `0x50d6d0` pitch-accumulate + clamp (`0x50d6d0`..`0x50d715`): the source is `0.0` when the
/// `[ecx+0x90]` bit-3 flag is set (`fld [0x7ffd74]`) else `[ecx+0x10c]` (`fld [ecx+0x10c]`); then
/// `fadd [ecx+0x40]` (the only owned arithmetic) and clamp into `[lo=0.0, hi=3.1241393]`:
/// `fcom lo; test ah,5; jp` then `fcom hi; test ah,5; jnp`.
///
/// The clamp uses `fcom` (NO pop), so the live PC_53 sum feeds both compares and is the returned
/// value when in range. **PF=1 ⟺ EVEN set-bits** of `{C0,C2}`:
/// * `fcom(sum, lo)`: `sum < lo` → C0=1 (odd) → `jp` NOT taken → `fstp st(0); fld lo` ⇒ return `lo`.
///   `sum >= lo` / NaN → even → `jp` taken → continue to the hi compare.
/// * `fcom(sum, hi)`: `sum < hi` → C0=1 (odd) → `jnp` NOT taken → fall to `ret` with live `sum`
///   ⇒ return `sum`. `sum >= hi` / NaN → even → `jnp` taken → `fstp st(0); fld hi` ⇒ return `hi`.
///
/// Net: `sum < 0 → 0; sum >= hi → hi; else → sum`. NaN routes to `lo` first (the `jp`-even branch
/// goes to the hi compare, where NaN is even again → `hi`) — i.e. NaN → `hi`, mirroring the
/// hardware's unordered-sets-C0/C2 disposition. Returned in st0 (the caller reloads as f32).
pub fn pitch_clamp_accum(source: f32, accum: f32, flag_bit3: bool) -> f32 {
    let src = if flag_bit3 { 0.0 } else { f64::from(source) };
    let sum = src + f64::from(accum); // the owned fadd (PC_53)
    let lo = f64::from(f32::from_bits(PITCH_LO_BITS)); // 0.0
    let hi = f64::from(f32::from_bits(PITCH_HI_BITS)); // 3.1241393
    use core::cmp::Ordering;
    // sum < lo (ordered) ⇒ lo; the `jp` (odd ⇒ <) reconstruction.
    if matches!(sum.partial_cmp(&lo), Some(Ordering::Less)) {
        lo as f32
    } else if matches!(sum.partial_cmp(&hi), Some(Ordering::Less)) {
        sum as f32 // ordered-below hi ⇒ in range, live sum
    } else {
        hi as f32 // sum >= hi (ordered) OR unordered (NaN) ⇒ even-parity ⇒ hi
    }
}

// ===========================================================================================
// (4) 0x50d0f0 — percent·0.01 scale (the only owned arithmetic in the 546-byte body)
// ===========================================================================================

/// `0x50d0f0` alpha/percent scale (`0x50d1f3`..`0x50d1fc`): a byte field `[edx+0x1d]` is clamped to
/// `min(.,100)` (`cmp al,0x64; jb; mov al,0x64`), zero-extended to an i32, then `fild [ebp-4];
/// fmul [0x8029d0]=0.01; fstp [ebp-4]` → `clamp01_pct · 0.01` stored f32. This is the single owned
/// FP-arith op in the function (the rest is a 3-axis deadband `fcomp` — see [`deadband_eq_any3`] —
/// plus pure struct shuffling delegated to `0x511250`/`0x50d900`/`0x50dbc0`). `pct` is the byte
/// (0..=255 → clamped to 0..=100). Returns the f32 fraction stored to `[ebp-4]`.
pub fn percent_scale(pct: u8) -> f32 {
    let n = i32::from(pct.min(100));
    (f64::from(n) * hundredth()) as f32
}

/// `0x50d0f0` 3-axis deadband (`0x50d147`..`0x50d174`): three independent `fld [ecx+k]; fcomp
/// [edi+thr]; fnstsw; test ah,0x44; jp` tests (channels vs `[edi+0x174]/0x178/0x17c]`). mask 0x44 =
/// `{C2,C3}`; **PF=1 ⟺ EVEN set-bits**, so `jp` (PF=1) taken iff EVEN. After `fcomp(v,thr)`: C3=1
/// iff `v==thr`, C2=1 iff unordered. ordered `v!=thr` → 0 bits (even) → `jp` TAKEN → `ecx=1`
/// (differs). `v==thr` → C3=1 (odd) → not taken → next axis. NaN → C2=C3=1 (2 bits, even) → taken →
/// `ecx=1`. The third axis uses `jnp` (the inverse) onto the `ecx=0` join. Net: `ecx=1` iff ANY
/// axis `v != thr` (ordered) or unordered; `ecx=0` iff ALL three `v == thr` exactly. (No FP-arith —
/// `fcomp`-only — but the family's parity branch, diffed for completeness.) Returns `true` ⇒ ecx=1.
pub fn deadband_eq_any3(v: [f32; 3], thr: [f32; 3]) -> bool {
    for i in 0..3 {
        let a = f64::from(v[i]);
        let t = f64::from(thr[i]);
        if a.is_nan() || t.is_nan() || a != t {
            return true;
        }
    }
    false
}

// ===========================================================================================
// (5) 0x50dbc0 — rate products (fmul×3) + |·| eps test (fabs) + clamp-to-[lo,hi] tail
// ===========================================================================================

/// `0x50dbc0` step product (`0x50dcb9`..`0x50dccb`): `step = src0x24 · field0xa0`
/// (`fld [eax+0x24]; fmul [esi+0xa0]; fstp [ebp-4]`), stored f32 to `[ebp-4]`. `eax`/`esi+0xa0`
/// come from the resolved table-row struct (`[eax+0xbe0d00]`) and the `this` rate field; here we
/// expose the owned `fmul`. The first of the function's four FP-arith ops.
pub fn step_mul(src: f32, rate: f32) -> f32 {
    (f64::from(src) * f64::from(rate)) as f32
}

/// `0x50dbc0` |step| vs eps probe (`0x50dcf8`..`0x50dd08`): `fld [ebp+8]; fabs; fcomp [0x8029d4]=
/// eps(2⁻²²); fnstsw; test ah,5; jnp 0x50dd86`. `[ebp+8]` holds the f32 `[ecx+0x24]` (a per-channel
/// field). **PF=1 ⟺ EVEN**, so `jnp` (PF=0) taken iff ODD. ordered `|v|<eps`: C0=1 (1 bit, odd) →
/// `jnp` TAKEN → `0x50dd86` (the "settled — write back directly" path). `|v|>=eps`/==/NaN → even →
/// not taken → the clamp+publish tail. Returns `true` ⇒ `|v| < eps` (false on NaN) ⇒ the binary
/// takes the early settle path (skips the clamp). The `fabs` is the function's second FP-arith op.
pub fn abs_lt_eps(field: f32) -> bool {
    let m = f64::from(field).abs();
    !m.is_nan() && m < f64::from(f32::from_bits(EPS_BITS))
}

/// `0x50dbc0` clamp tail (`0x50dd2e`..`0x50dd83`): a `[lo, hi)` clamp of the current `[esi+0x1ac]`
/// value, where the two bounds are products (load order from the two table globals):
/// * `lo = bLo · m` (`fld [0xbe1050]+0x24; fmul [ebp+8]=m`, kept LIVE on the x87 stack), and
/// * `hi = bHi · m` (`fld [0xbe1054]+0x24; fmul [ebp+8]=m; fstp [ebp+0x10]`) — f32 in `[ebp+0x10]`.
///   These are the function's third and fourth FP-arith ops (two `fmul`s).
///
/// Then (`0x50dd43`..): `fld [esi+0x1ac]=field; fst [ebp+8]` (**store NO pop** — live-st0
/// trap: `[ebp+8]` now holds the f32-rounded field, while st0 stays the live PC_53 field);
/// `fcomp st(1)=lo` (pops field, leaving `lo` live in st0). **PF=1 ⟺ EVEN set-bits** of the masked
/// `{C0,C2}`, so `jnp` (PF=0) is taken on ODD, `jp` (PF=1) on EVEN. Reconstructed from the EMITTED
/// jccs and their exact landings:
/// * `0x50dd53 jnp 0x50dd78`: taken iff `field < lo` (live, C0=1 → odd). 0x50dd78 SKIPS the
///   `fld [ebp+0x10]` and stores the **live st0 = `lo`** ⇒ return `lo`. (So the lo compare reads the
///   LIVE f64 field; a below-lo value clamps to `lo`.)
/// * `field >= lo` (live) OR NaN → even → not taken → `fstp st(0)` (drop `lo`); `fld [ebp+8]` = the
///   **f32** field; `fcomp [ebp+0x10]=hi`; `0x50dd62 jp 0x50dd75`: taken iff EVEN:
///   * `field_f32 >= hi` / NaN → even → `jp` TAKEN → `fld [ebp+0x10]` ⇒ return `hi`.
///   * `field_f32 < hi` (ordered, C0=1 → odd) → not taken → `fld [ebp+8]` ⇒ return the **f32** field.
///
/// Net `[lo, hi)` clamp: `field < lo (live f64) → lo`; else `field_f32 >= hi → hi`; else
/// `field_f32` (the f32-rounded field). The asymmetric rounding is load-bearing: the lo compare
/// uses the live PC_53 field, the hi compare + the passthrough use the `fst`-no-pop f32 copy. Result
/// `fstp [esi+0x1ac]` → f32. `field` is the live `[esi+0x1ac]` value; `b_lo`/`b_hi` the two `+0x24`
/// table fields (`[0xbe1050]`/`[0xbe1054]`); `m` the `[ebp+8]` multiplier (the f32 `[ecx+0x24]`).
pub fn clamp_tail(field: f32, b_lo: f32, b_hi: f32, m: f32) -> f32 {
    use core::cmp::Ordering;
    let lo = f64::from(b_lo) * f64::from(m); // live on the x87 stack (f64) — the `fcomp st(1)` operand
    let hi = (f64::from(b_hi) * f64::from(m)) as f32; // fstp [ebp+0x10] → f32
    let field_f64 = f64::from(field); // live PC_53 field (the lo compare reads this)
    let field_f32 = field; // the fst-no-pop f32 reload (hi compare + passthrough)

    // lo compare: field < lo (live, ordered) ⇒ lo. (jnp odd ⇒ <.)
    if matches!(field_f64.partial_cmp(&lo), Some(Ordering::Less)) {
        lo as f32
    } else if matches!(
        f64::from(field_f32).partial_cmp(&f64::from(hi)),
        Some(Ordering::Less)
    ) {
        // field_f32 < hi (ordered) ⇒ jp NOT taken ⇒ the f32 field (in range).
        field_f32
    } else {
        hi // field_f32 >= hi / NaN ⇒ jp taken ⇒ hi
    }
}
