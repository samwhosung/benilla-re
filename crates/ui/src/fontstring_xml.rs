//! Reimplements the arithmetic of the font-string functions from `0x770570` to
//! `CSimpleFontString::SetTextHeight` (`0x771600`), including `CSimpleFontString::LoadXML`
//! (`0x770f40`). The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the
//! `fstp dword` stores; the tests in `tests/fontstring_xml.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

/// norm-ε = 2⁻²² (`DAT_008029d4`), the change-detect epsilon (shared with the camera glue family).
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07
/// 1024.0 (`DAT_007ffd68`), the unit-convert scale numerator of `0x770720`/`0x770790`/`0x770f40`.
pub const SCALE1024_BITS: u32 = 0x4480_0000;
/// 0.0 (`DAT_007ffd74`), the non-zero guard constant.
pub const ZERO_BITS: u32 = 0x0000_0000;

#[inline]
fn eps() -> f64 {
    f64::from(f32::from_bits(EPS_BITS))
}

// ===========================================================================================
// (1) lerp-by-|delta| field update — 0x770570 (two parallel blocks, the pure owned leaf)
// ===========================================================================================

/// `0x770570` block 1 (`0x770573`..`0x77058f`):
/// `fld [ecx+0x114]; fsub [ecx+0x104]; fabs; fld [eax+0xc]; fsub [eax+0x4]; fmulp; fadd [eax+0x4];
/// fstp [eax+0xc]`. So `[eax+0xc] = |a − b| · (hi − lo) + lo`, a lerp of the `[eax+0xc]..[eax+0x4]`
/// pair by the absolute change `|[ecx+0x114] − [ecx+0x104]|`. The whole PC_53 chain is `f64`,
/// narrowed to `f32` only at the `fstp dword` store. `a`=`[ecx+0x114]`, `b`=`[ecx+0x104]`,
/// `hi`=`[eax+0xc]` (the field overwritten), `lo`=`[eax+0x4]`.
///
/// Block 2 (`0x770592`..`0x7705a9`) is the byte-identical shape on the other offsets
/// (`a`=`[ecx+0x110]`, `b`=`[ecx+0x108]`, `hi`=`[eax+0x8]`, `lo`=`[eax]`) — one kernel covers both.
pub fn lerp_by_abs_delta(a: f32, b: f32, hi: f32, lo: f32) -> f32 {
    let frac = (f64::from(a) - f64::from(b)).abs();
    let r = frac * (f64::from(hi) - f64::from(lo)) + f64::from(lo);
    r as f32
}

// ===========================================================================================
// (2) cvar unit-convert kernel — 0x770720 / 0x770790 / 0x770f40 (byte-identical clone, disp-only)
// ===========================================================================================

/// The owned unit-convert kernel shared (byte-identical modulo the `fdivr` stack-disp) by
/// `0x770720` (`0x77076b`..`0x770775`), `0x770790` (`0x7707dc`..`0x7707e6`), `0x770f40`
/// (`0x77127a`..`0x771284`):
/// `fmul [0x7ffd68]=1024.0; fdivr [m]; fstp [esp]` with `st0` on entry = a scale getter's return
/// (`0x41ad70` = `[0x832a4c]`, or `0x64aaa0` = a cvar value). `fdivr` computes `dividend / st0`, so
/// the result is `dividend / (scale · 1024.0)`. The dividend is the f32-stored int from the
/// preceding `fild qword; fstp [m]` (0x770720/0x770790) or the cvar f32 (0x770f40). All ops are
/// PC_53 `f64`; the `fstp dword [esp]` narrows to `f32` (the value pushed to the next call).
pub fn cvar_unit_convert(dividend: f32, scale: f32) -> f32 {
    let denom = f64::from(scale) * f64::from(f32::from_bits(SCALE1024_BITS));
    (f64::from(dividend) / denom) as f32
}

// ===========================================================================================
// (3) eps change-detect predicate — fld[field]; fsubr[other]; fabs; fcomp[eps]; test ah,5; jXp
// ===========================================================================================

/// The family's repeated eps change-detect, shared by `0x770910` (`0x77097c`..`0x77098a`),
/// `0x770ad0` (`0x770b29`..`0x770b37`), `0x7714f0` (`0x77151f`..`0x771530`), `0x771600`
/// (`0x771606`..`0x771617`): `fld [field]; fsubr [other]; fabs; fcomp [0x8029d4]=2⁻²²; fnstsw`.
/// `fsubr` computes `other − field`. After `fcomp(|d|, eps)`: `C0=1 ⟺ |d|<eps`, `C2=1 ⟺ unordered`.
///
/// `test ah,5` masks `{C0,C2}`; the EMITTED jcc reads PARITY: odd parity ⟺ ordered
/// `|d| < eps`. ordered `|d|<eps`: C0=1,C2=0 → 1 bit (odd). ordered `|d|>=eps` / `==eps` (C3, not
/// masked): 0 bits (even). NaN: C0=C2=1 → 2 bits (even). So this predicate returns `true` ⟺
/// `|other − field| < eps` (false on NaN). The consuming site's jcc direction (`jp` at 0x770910/
/// 0x770ad0/0x7714f0, `jnp` at 0x771600) only flips which branch "settled" routes to; the math is
/// the same.
pub fn settled_eps(field: f32, other: f32) -> bool {
    let d = (f64::from(other) - f64::from(field)).abs();
    !d.is_nan() && d < eps()
}

/// The `fld [edi+0x18]; fld [esi+0x18]; fsubr st,st(1); fabs; fcomp [eps]` two-register variant
/// (`0x770a84`..`0x770a94` in `0x770910`, `0x770c1a`..`0x770c2a` in `0x770ad0`). `fsubr st,st(1)`
/// computes `st1 − st0` = `[edi+0x18] − [esi+0x18]` = `src − dst` — identical predicate to
/// [`settled_eps`] with `field`=dst, `other`=src. Kept as a named alias so the difftest covers the
/// `fsubr st,st(1)` opcode (`d8 e9`) explicitly distinct from the memory-operand `fsubr` (`d8 6d`).
pub fn settled_eps_st(dst: f32, src: f32) -> bool {
    settled_eps(dst, src)
}

// ===========================================================================================
// (4) scale-multiply — 0x7714f0 fmul [esi+0x7c]
// ===========================================================================================

/// `0x7714f0` scale-multiply (`0x77155d`..`0x771569`): `fld [ebp+0xc]; fmul [esi+0x7c]; fstp [esp]`
/// — `arg · field` → `f32` (the value pushed to the `0x44d040` handler). PC_53 product, `fstp
/// dword` narrows to `f32`.
pub fn scale_mul(arg: f32, field: f32) -> f32 {
    (f64::from(arg) * f64::from(field)) as f32
}
