//! Reimplements the arithmetic of the UI scale functions: the aspect getter `GetConfiguredAspect`
//! (`0x4901e0`), the scale setter `0x4905e0`, `UiScaleChanged` (`0x490770`), `UseUiScaleChanged`
//! (`0x4907f0`) and the position code of `CGGameUI::Initialize` (`0x48fbf0`). The x87 runs at
//! 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests
//! in `tests/uiscale.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// 1024.0 (`DAT_007ffd68`), the device-units-per-logical scale of `0x4905e0`'s converter.
pub const K1024_BITS: u32 = 0x4480_0000;
/// 3.0 (`DAT_0080297c`), the eps-gate numerator factor of `0x490770`/`0x4907f0`.
pub const THREE_BITS: u32 = 0x4040_0000;
/// 4.0 (`DAT_0080306c`), the eps-gate threshold of `0x490770`/`0x4907f0`.
pub const FOUR_BITS: u32 = 0x4080_0000;
/// 0.75 (`DAT_008012cc`), the scaled-position factor of `0x490770`/`0x4907f0`.
pub const ZP75_BITS: u32 = 0x3f40_0000;

#[inline]
fn k1024() -> f64 {
    f64::from(f32::from_bits(K1024_BITS))
}
#[inline]
fn three() -> f64 {
    f64::from(f32::from_bits(THREE_BITS))
}
#[inline]
fn four() -> f64 {
    f64::from(f32::from_bits(FOUR_BITS))
}
#[inline]
fn zp75() -> f64 {
    f64::from(f32::from_bits(ZP75_BITS))
}

// ===========================================================================================
// (1) 0x48fbf0 — Lua-registration driver. Owned FP: two finite-difference subtracts.
// ===========================================================================================

/// `0x48fbf0`'s owned finite-difference subtracts (`0x4900db`..`0x490106`): after a getter fills a
/// vector-shaped struct at `[ebp-0x2c..]`, two coordinate deltas are computed and `__ftol`'d:
/// * `dx = [ebp-0x20] − [ebp-0x28]` (`fld [ebp-0x20]; fsub [ebp-0x28]`),
/// * `dy = [ebp-0x24] − [ebp-0x2c]` (`fld [ebp-0x24]; fsub [ebp-0x2c]`).
///
/// Each difference is left LIVE on st0 (NOT stored), fed directly to the CRT `__ftol` (`0x40a2b0`,
/// truncate toward zero) → i32. The `__ftol` conversion is the CRT boundary; here we own only the
/// two `fsub`s and expose their truncated-to-i32 results (what the binary passes onward).
/// `a`/`b` = `[ebp-0x20]`/`[ebp-0x28]` (the dx pair); `c`/`d` = `[ebp-0x24]`/`[ebp-0x2c]` (dy).
/// Returns `(dx_i32, dy_i32)`.
pub fn finite_diff_ftol(a: f32, b: f32, c: f32, d: f32) -> (i32, i32) {
    let dx = f64::from(a) - f64::from(b); // fld [ebp-0x20]; fsub [ebp-0x28]
    let dy = f64::from(c) - f64::from(d); // fld [ebp-0x24]; fsub [ebp-0x2c]
    (dx.trunc() as i32, dy.trunc() as i32) // __ftol = truncate toward zero
}

// ===========================================================================================
// (2) 0x4901e0 — aspect-ratio (int numerator / int denominator).
// ===========================================================================================

/// `0x4901e0`'s only owned FP-arith (`0x490237`..`0x49023e`): `ratio = (f64)num / (i32)den`
/// (`fild dword [ebp-0xc]; fidiv dword [ebp-0x8]`). `num`/`den` are i32 device-dimension fields
/// (`[ebp-0xc]` defaults 4, `[ebp-0x8]` defaults 3, both overwritten when the device-mode query
/// `0x73fb5d` succeeds). `fild` widens `num` to f64 exactly; `fidiv` loads the i32 memory operand,
/// widens it, and divides st0 by it — at PC_53. The quotient is left LIVE on st0 (the caller
/// `fstp`'s it to `[0xb4e30c]`). Returns the f64 ratio. `num`/`den` the two i32 dimensions.
pub fn aspect_ratio(num: i32, den: i32) -> f64 {
    // fild widens num→f64; fidiv loads `den` (i32), widens, divides — PC_53 throughout.
    (num as f64) / (den as f64)
}

// ===========================================================================================
// (3) 0x4905e0 — cvar→device-position convert (4× byte-identical loop bodies).
// ===========================================================================================

/// `0x4905e0`'s per-axis device-position convert, the body shared 4× (`0x490632`..`0x49063c` and
/// its three byte-twins at `0x490653`/`0x4906af`/`0x4906e2`). After `fild [int_param]; fstp` stores
/// the int param as an f32, the device-scale getter `0x41ad70` returns the logical-units scale `s`
/// in st0, then:
/// * `prod = s · 1024.0` (`fmul [0x7ffd68]`),
/// * `pos = param_f32 / prod` (`fdivr [ebp+?]` = `mem / st0` = `param_f32 / (s·1024)`),
///
/// `fstp`'d → f32 (pushed to the layout setter `0x767c70` / a vtable setter). `param` is the int
/// pixel coordinate (already `fild`+`fstp`'d to f32 — so the dividend enters as f32); `s` is the
/// delegated device-scale (a getter result, supplied here). Returns the f32 device position.
pub fn device_position(param: i32, s: f32) -> f32 {
    let param_f32 = param as f32; // fild [int]; fstp → f32
    let prod = f64::from(s) * k1024(); // fmul [0x7ffd68]=1024.0
    (f64::from(param_f32) / prod) as f32 // fdivr [param_f32] = param_f32 / prod; fstp → f32
}

// ===========================================================================================
// (4) 0x490770 / 0x4907f0 — eps-gated scaled-position commit (shared owned shape).
// ===========================================================================================

/// `0x490770`/`0x4907f0`'s first eps-gate product (`0x490795` / `0x490815`): `g·3.0` where
/// `g = [0xb4e30c]` (the aspect ratio set by `0x4901e0`), compared against `4.0`
/// (`fmul [0x80297c]=3.0; fcomp [0x80306c]=4.0; test ah,5; jp`). mask 5 = {C0,C2}; `jp` (PF=1 ⟺
/// EVEN set-bits) taken iff EVEN. `fcomp(g·3, 4)` sets C0 iff `g·3 < 4`, C3 iff `g·3 == 4` (C3 is
/// bit 0x40, NOT in the mask). ordered `g·3 < 4` → masked {C0} = 1 bit (odd) → jp NOT taken;
/// `g·3 == 4` → masked 0 bits (even) → jp TAKEN; `g·3 > 4` → masked 0 bits (even) → jp TAKEN;
/// unordered → masked {C0,C2} = 2 bits (even) → jp TAKEN. Returns `true` ⇒ jp taken (the gate
/// short-circuits). `g` is the aspect ratio. (The product `g·3.0` is the owned arithmetic; the
/// branch is reconstructed from the EMITTED `jp`.)
pub fn gate_g3_ge4(g: f32) -> bool {
    use core::cmp::Ordering;
    let p = f64::from(g) * three();
    // jp taken ⇔ EVEN parity of {C0,C2}. ordered-below 4 (C0 alone) ⇒ odd ⇒ NOT taken; everything
    // else — `>= 4` ordered (incl `== 4`) OR unordered/NaN (the even-parity jp branch) ⇒ TAKEN.
    !matches!(p.partial_cmp(&four()), Some(Ordering::Less))
}

/// `0x490770`/`0x4907f0`'s second eps-gate product (`0x4907ae` / `0x490831`): `p = g·0.75` compared
/// against the live committed value `v` (`fmul [0x8012cc]=0.75` → st0=p; `fld [ebp+8]=v` → st0=v,
/// st1=p; `fcomp st(1)` → compares st0=v with st1=p, i.e. `fcomp(v, p)`; `fnstsw; test ah,5; jnp`).
/// `fcomp(v, p)` sets C0 iff `v < p`, C3 iff `v == p` (C3 = bit 0x40, NOT in the mask). mask 5 =
/// {C0,C2}; `jnp` (PF=0 ⟺ ODD set-bits) taken iff ODD. ordered `v < p` → {C0} 1 bit (odd) → jnp
/// TAKEN → drops st0 (the candidate is left as `v`). ordered `v > p` (0 bits, even) / `v == p` (C3,
/// not masked → 0 bits, even) / unordered (C0|C2, 2 bits, even) → jnp NOT taken → `fstp [ebp+8]`
/// stores `p = g·0.75` (committing the clamped value). Returns `true` ⇒ jnp taken (`v < g·0.75`,
/// candidate stays `v`). `g` aspect ratio, `v` the committed candidate. (Owned arithmetic =
/// `g·0.75`; branch reconstructed from the EMITTED `jnp` AND the `fcomp st(1)` operand order.)
pub fn gate_v_lt_g075(g: f32, v: f32) -> bool {
    let p = f64::from(g) * zp75();
    // jnp taken ⇔ ODD parity of {C0,C2}: v<p ⇒ 1 bit (odd) ⇒ taken; v>p / v==p / NaN ⇒ even.
    !v.is_nan() && !p.is_nan() && f64::from(v) < p
}

/// `0x4907f0`'s fallback finite-difference (`0x49088f`..`0x4908a4`, taken when the cvar lookup
/// `0x64ac60` fails / `[0xb4e2b0]` is null): a getter zero-fills a struct at `[ebp-0x10..]`, then
/// * `dx = [ebp-0x8] − [ebp-0x10]` (`fld [ebp-0x8]; fsub [ebp-0x10]`),
/// * `dy = [ebp-0x4] − [ebp-0xc]` (`fld [ebp-0x4]; fsub [ebp-0xc]`),
///
/// each LIVE st0 → `__ftol` (`0x40a2b0`, truncate toward zero) → i32, passed to `0x492f70`. Same
/// shape as [`finite_diff_ftol`] but distinct slot offsets / a distinct call site. `a`/`b` =
/// `[ebp-0x8]`/`[ebp-0x10]`; `c`/`d` = `[ebp-0x4]`/`[ebp-0xc]`. Returns `(dx_i32, dy_i32)`.
pub fn fallback_diff_ftol(a: f32, b: f32, c: f32, d: f32) -> (i32, i32) {
    let dx = f64::from(a) - f64::from(b); // fld [ebp-0x8]; fsub [ebp-0x10]
    let dy = f64::from(c) - f64::from(d); // fld [ebp-0x4]; fsub [ebp-0xc]
    (dx.trunc() as i32, dy.trunc() as i32) // __ftol = truncate toward zero
}
