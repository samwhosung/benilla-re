//! Reimplements the arithmetic of the camera view and input functions between
//! `CCamera::UpdateTransitions` (`0x50f160`) and `CCamera::ArmAutoReturn` (`0x510960`): view
//! store and reset, zoom, mouse turn, facing commit, and the bobbing and pivot-bias gates. The
//! x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores;
//! the tests in `tests/camera_view_input.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

/// dt = 0.001 (`DAT_00801360`), the ms→s tick scale.
pub const DT_BITS: u32 = 0x3a83_126f; // 0.0010000000474974513
/// 1.0 (`DAT_007ff9d8`), the clamp ceiling.
pub const ONE_BITS: u32 = 0x3f80_0000;
/// norm-ε = 2⁻²² (`DAT_008029d4`), the change-detect epsilon.
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07
/// π/180 (`DAT_007ffaac`), deg→rad.
pub const DEG2RAD_BITS: u32 = 0x3c8e_fa35; // 0.01745329238474369
/// 1000.0 (`DAT_008022cc`), the s→ms / unit scale of the `__ftol` converters.
pub const SCALE1000_BITS: u32 = 0x447a_0000;
/// 0.01 (`DAT_008029d0`).
pub const HUNDREDTH_BITS: u32 = 0x3c23_d70a;
/// 0.0 (`DAT_007ffd74`).
pub const ZERO_BITS: u32 = 0x0000_0000;
/// 1/6 (`DAT_008089ac`), the `cameraBobbing` gate's zoom-distance threshold.
pub const SIXTH_BITS: u32 = 0x3e2a_aaab; // 0.1666666716337204
/// ±89° pitch clamp, the camera vertical limit (`DAT_008089d8` = +1.5533430576, `..dc` = −).
pub const PITCH_CLAMP_BITS: u32 = 0x3fc6_d3f2; // 1.5533430576324463

#[inline]
fn dt() -> f64 {
    f64::from(f32::from_bits(DT_BITS))
}
#[inline]
fn deg2rad() -> f64 {
    f64::from(f32::from_bits(DEG2RAD_BITS))
}

// ===========================================================================================
// (1) rate-scaled tick-delta fraction + clamp — 0x50f160 per-channel (8×), shared shape
// ===========================================================================================

/// The per-channel rate fraction of `0x50f160` (e.g. `0x50f18d`..`0x50f19f`):
/// `frac = (i64)(now − prev_tick) · dt(0.001) / rate`, computed as a PC_53 chain
/// (`fild qword; fmul [0x801360]; fdiv [rate]`). `now`/`prev` are i32 tick counts; the i64 widen is
/// `fild qword` of `{Δ, 0}` (the high dword is forced to 0 at `0x50f18a`), so Δ reads as the i32
/// value zero-/sign-extended exactly — here the low dword is `now−prev` as a 32-bit signed value
/// stored then read as the low half with high=0, i.e. the **unsigned** 32-bit magnitude of the
/// wrapped difference, zero-extended to i64 (matching the `mov [ebp-8],eax; mov [ebp-4],0`).
pub fn rate_frac(now: i32, prev: i32, rate: f32) -> f64 {
    let delta = now.wrapping_sub(prev) as u32; // [ebp-8] = eax (Δ), [ebp-4] = 0
    (i64::from(delta) as f64) * dt() / f64::from(rate)
}

/// The clamp gate of `0x50f160` (`0x50f19c`..`0x50f1aa`): `fst [ebp-4]` stores `frac` as f32 (st0
/// stays LIVE) then `fcomp 1.0; test ah,1; jne` takes the interp path iff `C0` set, i.e. iff the
/// **live f64** `frac < 1.0` (NaN ⇒ C0 set ⇒ treated as <1.0). Returns `true` when the channel
/// interpolates (frac < 1.0), `false` when it snaps (frac ≥ 1.0).
pub fn rate_frac_interpolates(frac: f64) -> bool {
    // partial-order `<`: false on NaN would be WRONG here — the binary's C0 is set on unordered,
    // so an explicit NaN check routes NaN to the interp path exactly like the hardware.
    frac.is_nan() || frac < 1.0
}

/// The f32 the interp (`0x5b7bb0`) actually receives — the `fst dword [ebp-4]` value reloaded.
pub fn rate_frac_f32(frac: f64) -> f32 {
    frac as f32
}

// ===========================================================================================
// (2) eps change-detect predicates — fld/fsub(r); fabs; fcomp [eps]; test ah,MASK; jcc
// ===========================================================================================

/// `0x50f160` per-channel change-detect (`0x50f20b`..`0x50f22a`):
/// `fld [target]; fsub [field]; fabs; fcomp [eps=2⁻²²]; fnstsw; test ah,5; jnp 0x50f295(=skip)`.
/// `test ah,5` masks {C0,C2}; **PF=1 ⟺ EVEN set-bits**, so `jnp` (PF=0) is taken iff ODD set-bits.
/// After `fcomp(|d|, eps)`: C0=1 iff `|d|<eps`. ordered `|d|<eps`: C0=1,C2=0 → 1 bit (odd) → jnp
/// TAKEN → SKIP (the channel is settled). ordered `|d|>eps`: 0 bits (even) → not taken → does the
/// work. `|d|==eps`: C3=1, masked=0 (even) → work. NaN: C0=C2=1 → 2 bits (even) → work. So the
/// channel SKIPS (settled) iff `|target−field| < eps` (ordered). Returns `true` ⇒ `|d| < eps`
/// (false on NaN) ⇒ the binary SKIPS this channel.
pub fn changed_lt_eps(target: f32, field: f32) -> bool {
    let d = (f64::from(target) - f64::from(field)).abs();
    !d.is_nan() && d < f64::from(f32::from_bits(EPS_BITS))
}

/// `0x5106f0` (`0x5106f3`..`0x510712`): `fld [+0x118]; fadd [+0x114]; fadd [+0x110]; fabs;
/// fcomp dt(0.001); fnstsw; test ah,5; jp 0x510718(=body)`. **PF=1 ⟺ EVEN set-bits**, so `jp`
/// (PF=1) is taken iff EVEN. After `fcomp(|sum|,dt)`: `|sum|<dt` → C0=1 (1 bit, odd) → jp NOT
/// taken → `xor eax,eax; ret 0`. `|sum|>=dt`/`==dt` → 0 bits (even) → jp taken → body. NaN →
/// C0|C2=5 (2 bits, even) → body. So the **body fires** iff `|sum| >= dt` (i.e. NOT `|sum|<dt`).
/// This function is the predicate `|sum| < dt` (false on NaN) — the binary takes the BODY iff this
/// is false. Summation order `(a + b) + c`, a=[+0x118] b=[+0x114] c=[+0x110].
pub fn sum3_abs_lt_dt(a118: f32, b114: f32, c110: f32) -> bool {
    let s = (f64::from(a118) + f64::from(b114)) + f64::from(c110);
    let m = s.abs();
    !m.is_nan() && m < dt()
}

/// `0x5107f0` (`0x5107f3`..`0x510806`): `fld [+0x104]; fabs; fcomp dt; test ah,5; jp 0x51080c(=body)`.
/// Same shape as `sum3_abs_lt_dt`: `jp` (PF=1, even) taken iff `|field| >= dt`/==/NaN → body. This
/// function is the predicate `|field| < dt` (false on NaN); the binary takes the body iff false.
pub fn abs_lt_dt(field104: f32) -> bool {
    let m = f64::from(field104).abs();
    !m.is_nan() && m < dt()
}

/// `0x510850` near-equal probe (`0x510910`..`0x510923`): a delegated getter returns a value in st0,
/// then `fsubr [+0xf0]; fabs; fcomp eps(2⁻²²); fnstsw; test ah,5; jnp 0x510930(=ret 0)` — `fsubr`
/// computes `[+0xf0] − ret`. **PF=1 ⟺ EVEN**, so `jnp` (PF=0) taken iff ODD. ordered `|d|<eps`:
/// C0=1 (1 bit, odd) → jnp TAKEN → 0x510930 → `xor eax,eax; ret 0`. `|d|>=eps`/==/NaN → even → not
/// taken → `mov eax,1; ret 1`. So the binary returns **0** iff `|field0xf0 − ret| < eps`. This
/// function is the predicate `|d| < eps` (false on NaN); the binary returns 0 iff this is true.
pub fn near_eq_eps(field0xf0: f32, delegated: f32) -> bool {
    let d = (f64::from(field0xf0) - f64::from(delegated)).abs();
    !d.is_nan() && d < f64::from(f32::from_bits(EPS_BITS))
}

// ===========================================================================================
// (3) deg→rad flag-gated convert — 0x50fae0 / 0x50fb80 / 0x50fbc0 channel arrays
// ===========================================================================================

/// `0x803a78` = 180/π = 57.295780 (`DAT_00803a78`), the rad→deg convert of `0x50f990`.
pub const RAD2DEG_BITS: u32 = 0x4265_2ee1; // 57.295780181884766
#[inline]
fn rad2deg() -> f64 {
    f64::from(f32::from_bits(RAD2DEG_BITS))
}

/// The flag-gated deg→rad convert shared by `0x50fae0` (`fmul[π/180]` @`0x50fb32`), `0x50fb80`
/// (@`0x50fba3`), `0x50fbc0` (@`0x50fbf2`): a source angle (from a getter or `[+0x24]`) is
/// multiplied by `π/180` IFF a per-channel flag (`[0x84f474]`) is nonzero, else passed through.
/// `flag_set` is that runtime flag. Result is `fstp`'d → **f32** at the store.
pub fn deg2rad_gated(angle: f32, flag_set: bool) -> f32 {
    let v = f64::from(angle);
    let r = if flag_set { v * deg2rad() } else { v };
    r as f32
}

/// `0x50f990` rad→deg flag-gated convert (`0x50f9d0`..`0x50f9e8`): `fld [ebx]; (flag) fmul
/// [0x803a78]=57.29578; fstp QWORD [esp]`. Unlike the deg→rad siblings the result is stored as an
/// **f64** (`fstp qword` — pushed to a `%g` formatter `0x64a7f0`), so NO f32 narrowing. Returns the
/// f64 the formatter receives.
pub fn rad2deg_gated(angle: f32, flag_set: bool) -> f64 {
    let v = f64::from(angle);
    if flag_set {
        v * rad2deg()
    } else {
        v
    }
}

/// `0x50fbc0` change-detect tail (`0x50fbf8`..`0x50fc07`): after the gated convert leaves the
/// converted value in st0, `fsubr [field]; fabs; fcomp dt; fnstsw; test ah,5; jp 0x50fc23(=continue
/// loop / next channel)`. `fsubr` = `field − converted`. **PF=1 ⟺ EVEN**, so `jp` (PF=1) taken iff
/// EVEN. ordered `|d|<dt`: C0=1 (1 bit, odd) → jp NOT taken → fall to `add esi,4 … jb` loop continue
/// path — no, the fall-through at `0x50fc09` advances the loop, and `0x50fc23` is the early `xor
/// eax,eax; ret 0`. So `|d|<dt` (odd, not taken) → loop CONTINUES; `|d|>=dt`/NaN (even, jp taken) →
/// `0x50fc23` ret 0 (early-out). Returns `true` ⇒ `|field − converted| < dt` (false on NaN) ⇒ the
/// binary CONTINUES (does not early-out) on this channel.
pub fn deg2rad_settled(field: f32, converted: f32) -> bool {
    let d = (f64::from(field) - f64::from(converted)).abs();
    !d.is_nan() && d < dt()
}

// ===========================================================================================
// (4) unit-convert via __ftol — 0x50fc60 / 0x50fcd0
// ===========================================================================================

/// `0x50fc60` / `0x50fcd0` (`0x50fc68`..`0x50fc79` / `0x50fcd8`..`0x50fce9`):
/// `fld [G+0x24]; fdivr arg; fmul [0x8022cc]=1000.0; __ftol`. `fdivr` = `arg / G`. The product
/// `(arg / G) · 1000.0` is then `__ftol`'d — **truncate toward zero** (the CRT `__ftol` sets
/// round-toward-zero in its own FPCW). Both functions are byte-identical in this slice (only the
/// downstream flag offsets differ), one kernel covers both. `g` = `*(*(0xbe0fbc)+0x24)`.
pub fn unit_convert_ftol(arg: f32, g: f32) -> i32 {
    let v = (f64::from(arg) / f64::from(g)) * f64::from(f32::from_bits(SCALE1000_BITS));
    // __ftol = truncate toward zero.
    v.trunc() as i32
}

// ===========================================================================================
// (5) ±89° pitch clamp-select — 0x50f710 / 0x510960 tail / 0x5103e0 / 0x50fee0
// ===========================================================================================

/// The ±89° pitch clamp-select (`0x5103ef`..`0x510447`, the verified copy; mirrored at the `0x50f710`
/// tail / `0x50fee0`). `sum` (= `[+0x104]+[+0xf4]` at 0x5103e0) is clamped to ±89°:
/// `fcom lo(−89°,0x8089dc); test ah,5; jp 0x51042c`. **PF=1 ⟺ EVEN**:
/// * `sum < lo` → C0=1 (odd) → jp NOT taken → result = `lo` (−89°);
/// * `sum >= lo` → 0 bits (even) → jp TAKEN → `fcom hi(+89°,0x8089d8); jp 0x51043e` → `sum < hi`
///   (odd, NOT taken) → result = `sum` (in range); `sum >= hi` (even, TAKEN) → result = `hi`.
///
/// Net: `sum < −89° → −89°; sum > +89° → +89°; else → sum`. NaN routes to the `lo` then `hi`
/// selects like the hardware (unordered sets C0). Result `fst`'d → f32.
pub fn pitch_clamp(angle: f32) -> f32 {
    pitch_clamp_f64(f64::from(angle)) as f32
}

/// The shared ±89° clamp on a PC_53 f64, mirroring the binary's two even-parity selects exactly:
/// `lo` iff `sum < lo` (ordered); else `hi` iff `sum` is NOT ordered-below `hi` (i.e. `sum >= hi`
/// ordered OR unordered/NaN — the even-parity `jp` branch); else pass through. Written with
/// `partial_cmp` so the NaN→`hi` routing is explicit (a plain `>=` would be false on NaN — wrong).
fn pitch_clamp_f64(sum: f64) -> f64 {
    use core::cmp::Ordering;
    let hi = f64::from(f32::from_bits(PITCH_CLAMP_BITS)); // +1.5533430576
    let lo = -hi;
    if matches!(sum.partial_cmp(&lo), Some(Ordering::Less)) {
        lo
    } else if matches!(sum.partial_cmp(&hi), Some(Ordering::Less)) {
        sum // ordered-below hi ⇒ in range
    } else {
        hi // >= hi (ordered) OR unordered (NaN) ⇒ even-parity ⇒ hi
    }
}

/// `0x5103e0` add-then-clamp (`0x5103ef`..`0x510447`): `sum = [+0x104] + [+0xf4]` (a PC_53 f64
/// chain, never narrowed before the clamp — the `fcom` reads the live st0), then [`pitch_clamp`]
/// selects ±89°, stored `fstp [ebp-4]` → f32. The owned `fadd` + the two clamp selects.
pub fn add_then_pitch_clamp(a104: f32, b_f4: f32) -> f32 {
    pitch_clamp_f64(f64::from(a104) + f64::from(b_f4)) as f32
}

/// `0x5103e0` slew-throttle scale (`0x5104ba`..`0x5104c3`): `fild [ebp-0xc]` (an int 0..=100, the
/// clamped `min(turn_rate_pct, 100)`) `· 0.01(0x8029d0)`, then `fcomp 0.0`. The owned `fmul[0.01]`.
/// The product is the fractional throttle; we expose it (stored nowhere — left live for the fcomp,
/// so the diffed slice reads the `fnstsw` AH at `0x5104c9` instead of a store). `n` = the int.
pub fn throttle_frac(n: i32) -> f64 {
    f64::from(n) * f64::from(f32::from_bits(HUNDREDTH_BITS))
}

// ===========================================================================================
// (6) 0x50f710 deadband — 3× per-axis fcomp against per-axis thresholds
// ===========================================================================================

/// `0x50f710` deadband (`0x50f752`..`0x50f78b`): three independent `fld [m32]; fcomp [thr]; fnstsw;
/// test ah,0x44; jp 0x50f786(=eax=1)` tests (axes vs `[+0x174]/[+0x178]/[+0x17c]`). mask 0x44 =
/// {C2,C3}; **PF=1 ⟺ EVEN**, so `jp` (PF=1) taken iff EVEN set-bits. After `fcomp(v,thr)`: C3=1 iff
/// `v==thr`, C2=1 iff unordered. ordered `v!=thr` → C2=0,C3=0 → 0 bits (even) → jp TAKEN → `eax=1`.
/// `v==thr` → C3=1 (odd) → not taken → next axis. NaN → C2=C3=1 (2 bits, even) → taken → eax=1. So
/// `eax=1` iff ANY axis `v != thr` (ordered) or unordered; `eax=0` iff ALL three `v == thr` exactly.
/// Returns `true` ⇒ eax=1 (the `0x50f786` "differs/moved" branch).
pub fn deadband_eq_any(v: [f32; 3], thr: [f32; 3]) -> bool {
    for i in 0..3 {
        let a = f64::from(v[i]);
        let t = f64::from(thr[i]);
        // jp taken (eax=1) iff this axis is NOT exactly equal (differs ordered, or unordered/NaN).
        if a.is_nan() || t.is_nan() || a != t {
            return true;
        }
    }
    false
}

// ===========================================================================================
// (7) 0x50fee0 mouse-look rate products + ±89° clamp integration
// ===========================================================================================

/// `0x50fee0` mouse-look turn/pitch rate products (`0x50ff32`..`0x50ff6f`), the owned scalar math
/// the camera-control note records as 0x50fee0's only fidelity arithmetic:
/// * `rate_pitch = ([ -0x14 ] · π/180) · (dy · cvarPitch[0x808ac4])` then negated by `sign` (a
///   `±1.0` chosen from a flag) — disasm: `fld [-0x14]; fmul [π/180]; fld dy; fmul [0x808ac4];
///   fmulp; fstp [-0x14]` with sign applied later at `0x510023` (`fld [-0x4]=sign; fmul [-0x8]`).
/// * `rate_yaw  = ([ -0x10 ] · π/180) · (dx · cvarYaw[0x808ac0])` — `fld [-0x10]; fmul [π/180];
///   fld dx; fmul [0x808ac0]; fmulp; fstp [-0x8]`.
///
/// Here we expose the two raw rate magnitudes (pre-sign): `g_pitch`/`g_yaw` are the frame-time/
/// turn-rate globals (`[-0x14]`/`[-0x10]` seeds, read from `[0xbe1094]+0x24`/`[0xbe0f30]+0x24`);
/// `dx`/`dy` the mouse deltas (`[ebp+8]`/`[ebp+0xc]`); cvars the two `0x808ac0/4` constants.
pub fn mouselook_rate_yaw(g_yaw: f32, dx: f32, cvar_yaw: f32) -> f32 {
    // ([-0x10] · π/180) · (dx · cvarYaw), fmulp into one term, fstp → f32.
    let term_a = f64::from(g_yaw) * deg2rad();
    let term_b = f64::from(dx) * f64::from(cvar_yaw);
    (term_a * term_b) as f32
}
pub fn mouselook_rate_pitch(g_pitch: f32, dy: f32, cvar_pitch: f32) -> f32 {
    let term_a = f64::from(g_pitch) * deg2rad();
    let term_b = f64::from(dy) * f64::from(cvar_pitch);
    (term_a * term_b) as f32
}

/// `0x50fee0` pitch-integrate + ±89° clamp (`0x510020`..`0x51002c` + `0x51004a`..`0x510079`):
/// `field0x104 = sign·rate_pitch + field0x104` (`fld [-0x4]=sign; fmul [-0x8]=rate; fadd [+0x104];
/// fst [+0x104]`), then if a flag is unset, clamp to the upper limit `[0x8089dc]+? − field`… the
/// note's clamp. Here we expose the single owned product+accumulate: `out = sign·rate + field`
/// (`fst`→f32 store).
pub fn pitch_integrate(sign: f32, rate: f32, field: f32) -> f32 {
    ((f64::from(sign) * f64::from(rate)) + f64::from(field)) as f32
}

/// `0x50fee0` sign·rate push (`0x5100b0`..`0x5100b9`): `fld [-0x4]=sign; fmul [-0x8]=rate; fstp
/// [esp]` — the pitch integrator argument `sign·rate` → f32 (PC_53 product). Also the generic
/// "two-operand f32 product" shape (`0x510b61` slope·frac in 0x510960's table interp).
pub fn mul2_f32(a: f32, b: f32) -> f32 {
    (f64::from(a) * f64::from(b)) as f32
}

/// `0x50fee0` negated yaw push (`0x5100ef`..`0x5100fa`): `fld [-0x10]; fmul [-0x14]; fchs; fstp
/// [esp]` — `−([-0x10]·[-0x14])` → f32 (the yaw integrator argument).
pub fn neg_mul2_f32(a: f32, b: f32) -> f32 {
    (-(f64::from(a) * f64::from(b))) as f32
}

/// Generic "two-operand f32 sum" shape: `0x510b6a` (`fadd [ebp-0x28]`, the table interp base+offset)
/// and `0x510f04` (`fadd [ebp+0xc]`, the delegated-value + arg pushed to sound). `fstp`→f32.
pub fn add2_f32(a: f32, b: f32) -> f32 {
    (f64::from(a) + f64::from(b)) as f32
}

/// `0x50fee0` upper-limit residual (`0x51004a`..`0x510056`): `fld [0x8089dc=−89°]; fsub [+0xf4];
/// fstp [-0xc]` — `(−89°) − field` → f32. (`const_lhs` = the loaded constant, `field` = the
/// subtrahend.)
pub fn sub_field(const_lhs: f32, field: f32) -> f32 {
    (f64::from(const_lhs) - f64::from(field)) as f32
}

/// `0x510960` table-clamp residual (`0x510ad1`..`0x510add` / `0x510ba1`..`0x510bae`): `fld
/// [table+0x808a1c]; fsub [0x7ff9d8=1.0]; fstp` — `pitch_hi − 1.0` → f32, the slope-table upper
/// bound's pre-decrement.
pub fn sub_one(table_val: f32) -> f32 {
    (f64::from(table_val) - 1.0) as f32
}

/// `0x50fee0` |slot| vs slot predicate (`0x50fff0`..`0x50fffd`): `fld [-0x8]; fabs; fcomp [-0xc];
/// fnstsw; test ah,0x41; jne 0x510013`. mask 0x41 = {C0,C3}; `jne` tests ZF (is masked-AH zero) —
/// **not** parity. After `fcomp(|a|, b)`: C0=1 iff `|a|<b`, C3=1 iff `|a|==b`. `jne` taken (masked
/// AH != 0) iff `|a|<b` OR `|a|==b` OR unordered → jumps to 0x510013 (skip). Falls through (does the
/// `mov ecx,1`) iff `|a| > b` (ordered, strictly). Returns `true` ⇒ `|a| > b` (the fall-through).
pub fn abs_gt(a: f32, b: f32) -> bool {
    let m = f64::from(a).abs();
    // ordered strictly-greater (false on NaN / equal / less).
    !m.is_nan() && !f64::from(b).is_nan() && m > f64::from(b)
}

/// `0x5105e0` first conjunct — the `cameraBobbing` gate's zoom-distance seam
/// (`0x5105e3`..`0x5105f5`): `fld [cam+0xec]; fcomp [0x8089ac]; fnstsw ax; test ah,0x41; je fail`.
/// mask `0x41` = {C0,C3} read by `je`, i.e. **ZF** (is the masked AH zero), not parity. After
/// `fcomp(dist, 1/6)`: C0=1 iff `dist < k`, C3=1 iff `dist == k`, both clear iff `dist > k`
/// (ordered), and a NaN sets C0=C2=C3. So `je` (masked AH == 0) is taken — and the gate FAILS —
/// only on an ordered strictly-greater. Returns `true` ⇒ the gate's conjunct HOLDS, i.e.
/// `dist <= 1/6` (**inclusive at the threshold**) or unordered.
pub fn dist_le_sixth(dist_0xec: f32) -> bool {
    let d = f64::from(dist_0xec);
    let k = f64::from(f32::from_bits(SIXTH_BITS));
    // NaN ⇒ C0=C2=C3 ⇒ masked AH != 0 ⇒ `je` not taken ⇒ the conjunct holds.
    d.is_nan() || d <= k
}

/// `0x510690` last conjunct — the `cameraPivot` gate's pitch seam (`0x5106c3`..`0x5106d4`):
/// `fld [cam+0xf4]; fcomp [0x7ffd74=0.0]; fnstsw ax; test ah,0x41; je fail`. Identical shape to
/// `dist_le_sixth`, against `0.0`. Returns `true` ⇒ the conjunct HOLDS, i.e. `pitch <= 0.0`
/// (**inclusive at exactly zero**) or unordered — the eye at or below the target (down is +89°).
pub fn pitch_le_zero(pitch_0xf4: f32) -> bool {
    let p = f64::from(pitch_0xf4);
    p.is_nan() || p <= f64::from(f32::from_bits(ZERO_BITS))
}
