//! Reimplements the arithmetic of the clip-rectangle functions `0x509520`, `0x509720`,
//! `0x509dd0`, `0x509ec0` and `0x509fe0`. The x87 runs at 53-bit precision, so each chain is
//! `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/clip_rect.rs` diff each
//! against the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn f(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// `DAT_007ffa24` = 0.5 (`0x3f000000`), the halving constant 0x509720 / 0x509ec0 multiply by.
pub const HALF_BITS: u32 = 0x3f00_0000;

// ===========================================================================================
// (1) 0x509720 — midpoint of two clip-rect locals: `[edi] = (m[-0x4] + m[-0xc]) · 0.5`.
//     Owned slice 0x509772..0x50978a: `fld [ebp-0x4]; fadd [ebp-0xc]; fmul [0.5]; fstp [edi]`.
//     (`[edi+4]` is a straight copy of `m[-0x10]`, no arith.) fadd ×1 + fmul ×1.
// ===========================================================================================

/// `0x509720`'s owned midpoint (`0x509772`..`0x509788`): the two locals `a = m[ebp-0x4]` and
/// `b = m[ebp-0xc]` (two of the four floats the prologue's `call 0x5097a0` produced — they were
/// fcomp'd against `[esi+..]` bounds just above, integer-only branch result) are summed and halved.
/// `out = (a + b) · 0.5` stored to `[edi]` as f32. Returns that f32.
pub fn midpoint(a: f32, b: f32) -> f32 {
    ((f64::from(a) + f64::from(b)) * f(HALF_BITS)) as f32
}

// ===========================================================================================
// (2) 0x509dd0 — two `global / axis-extent` ratios, each `__ftol`-truncated, then `(t1+1)·(t2+1)`.
//     Body slice (all four arith inline): `fld [esi+0xc]; fsub [esi+4]; fdivr [ebp-0x4]`(=G44);
//     __ftol;  `fld [esi]; fsub [esi+8]; fdivr st,st(1)`(=G48); __ftol;  EAX = (t1+1)·(t2+1).
//     The two `fmul · arg` live in callees 0x41ae60 (G44·1.0) / 0x41ae70 (G48·1.0); with arg=1.0
//     they pass the globals through unchanged. fsub ×2 + fdivr ×2.
// ===========================================================================================

/// `0x509dd0`'s owned ratio-product. The struct (`ecx`/`esi`) holds four floats — `a = m[esi]`,
/// `b = m[esi+4]`, `c = m[esi+0xc]`, `d = m[esi+8]`. Two runtime globals `g44 = [0x832a44]`,
/// `g48 = [0x832a48]` (fetched as `g·1.0` via `0x41ae60`/`0x41ae70`) are each divided by an axis
/// extent and truncated toward zero (`__ftol`):
/// * `t1 = trunc( g44 / (c - b) )`   (`fdivr [ebp-0x4]` = reverse: mem ÷ st0)
/// * `t2 = trunc( g48 / (a - d) )`   (`fdivr st,st(1)`  = st1 ÷ st0)
///
/// returning `(t1 + 1) · (t2 + 1)` (the binary's `inc edi; inc eax; imul`). All four FP ops chain at
/// PC_53 f64; the f32 mem operands widen on load; `__ftol` truncates the f64 ratio. `g44`/`g48` are
/// the f32 globals; `a,b,c,d` the struct floats. Returns the i32 product.
pub fn ratio_product(g44: f32, g48: f32, a: f32, b: f32, c: f32, d: f32) -> i32 {
    // m[ebp-0x4] = (f32)(g44·1.0) = g44, m[ebp-0x14]-equivalent g48 — both pass through 0x41ae*.
    let r1 = f64::from(g44) / (f64::from(c) - f64::from(b)); // fdivr [ebp-0x4]
    let t1 = r1.trunc() as i32; // __ftol (truncate toward zero)
    let r2 = f64::from(g48) / (f64::from(a) - f64::from(d)); // fdivr st,st(1)
    let t2 = r2.trunc() as i32; // __ftol
    t1.wrapping_add(1).wrapping_mul(t2.wrapping_add(1)) // inc edi; inc eax; imul eax,edi
}

// ===========================================================================================
// (3) 0x509ec0 — two-axis center/extent resolve. Ten owned FP ops across five `fld;fOP[m32];fstp`
//     slices (each operand an ebp-local or an `[arg]` field). V1 = vtable `[+0x1c]`, V2 = vtable
//     `[+0x20]` (coordinate getters); P0/P1 = the arg-pointer pair `[ebp+0xc]→[eax]/[eax+4]`;
//     G44/G48 = the `0x41ae*` globals; X/Y = the post-clamp locals `[ebp-0x1c]`/`[ebp-0x18]`.
//     fmul ×2 + fsub ×6 + fadd ×2.
// ===========================================================================================

/// `0x509ec0` slice A (`0x509ee0`..`0x509efd`): the two `·0.5` halvings.
/// * `h1 = V1 · 0.5`  (`fmul [0.5]` → `[ebp-0x4]`)
/// * `h2 = V2 · 0.5`  (`fmul [0.5]` → `[ebp-0x8]`)
///
/// `v1`/`v2` are the two vtable-getter floats. Returns `(h1, h2)` as f32.
pub fn halve_pair(v1: f32, v2: f32) -> (f32, f32) {
    let half = f(HALF_BITS);
    ((f64::from(v1) * half) as f32, (f64::from(v2) * half) as f32)
}

/// `0x509ec0` slice B (`0x509f12`..`0x509f37`): build three struct floats from the arg-pointer pair
/// `P0 = [eax]`, `P1 = [eax+4]` and the halved/raw getters `h1 = V1·0.5 = m[ebp-0x4]`,
/// `v2 = m[ebp-0xc]`. (The first store `[ebp-0x2c] = P1` is a plain copy, no arith.)
/// * `s1 = P0 - h1`   (`fld [eax]; fsub [ebp-0x4]`   → `[ebp-0x28]`)
/// * `s2 = P1 - v2`   (`fld [eax+4]; fsub [ebp-0xc]` → `[ebp-0x24]`)
/// * `s3 = h1 + P0`   (`fld [ebp-0x4]; fadd [eax]`   → `[ebp-0x20]`)
///
/// Returns `(s1, s2, s3)` as f32. (`h1` is `V1·0.5`; `v2` is the raw second getter.)
pub fn build_struct(p0: f32, p1: f32, h1: f32, v2: f32) -> (f32, f32, f32) {
    let s1 = (f64::from(p0) - f64::from(h1)) as f32; // fsub [ebp-0x4]
    let s2 = (f64::from(p1) - f64::from(v2)) as f32; // fsub [ebp-0xc]
    let s3 = (f64::from(h1) + f64::from(p0)) as f32; // fadd [eax]
    (s1, s2, s3)
}

/// `0x509ec0` slice C (`0x509f63`..`0x509f6c`): `d1 = G44 - h1` (`fld [ebp-0x10]; fsub [ebp-0x4]`,
/// stored to scratch `[ebp+0xc]` then fcomp'd). `g44` is the global getter, `h1 = V1·0.5`. f32.
pub fn extent_x(g44: f32, h1: f32) -> f32 {
    (f64::from(g44) - f64::from(h1)) as f32 // fsub [ebp-0x4]
}

/// `0x509ec0` slice D (`0x509fa6`..`0x509faf`): `d2 = G48 - h2` (`fld [ebp-0x14]; fsub [ebp-0x8]`,
/// stored to scratch then fcomp'd). `g48` is the global getter, `h2 = V2·0.5`. f32.
pub fn extent_y(g48: f32, h2: f32) -> f32 {
    (f64::from(g48) - f64::from(h2)) as f32 // fsub [ebp-0x8]
}

/// `0x509ec0` slice E (`0x509fd4`..`0x50a00a`): the final three stores from the post-clamp locals
/// `X = m[ebp-0x1c]`, `Y = m[ebp-0x18]` and the halved/raw getters `h1 = V1·0.5 = m[ebp-0x4]`,
/// `v2 = m[ebp-0xc]`. (The first `fld [ebp-0x18]; fstp [ebp-0x3c]` is a copy of `Y`, no arith.)
/// * `r1 = X - h1`   (`fld [ebp-0x1c]; fsub [ebp-0x4]` → `[ebp-0x38]`)
/// * `r2 = Y - v2`   (`fld [ebp-0x18]; fsub [ebp-0xc]` → `[ebp-0x34]`)
/// * `r3 = X + h1`   (`fld [ebp-0x1c]; fadd [ebp-0x4]` → `[ebp-0x30]`)
///
/// Returns `(r1, r2, r3)` as f32. (`h1` is `V1·0.5`; `v2` is the raw second getter.)
pub fn finalize(x: f32, y: f32, h1: f32, v2: f32) -> (f32, f32, f32) {
    let r1 = (f64::from(x) - f64::from(h1)) as f32; // fsub [ebp-0x4]
    let r2 = (f64::from(y) - f64::from(v2)) as f32; // fsub [ebp-0xc]
    let r3 = (f64::from(x) + f64::from(h1)) as f32; // fadd [ebp-0x4]
    (r1, r2, r3)
}

// ===========================================================================================
// (4) 0x509520 — CRect viewport clamp (SmartScreenRect).
//     The clamp ROUTING (which midpoint/bound each corner takes) is integer cmov on `fcomp`
//     results — composition over already-diffed glue (0x509720 midpoint / 0x509e20 geo); the
//     OWNED FP is the half-extents + the two bound subtractions + the three final corner
//     stores, 11 arith ops across 4 slices, each operand synthesized directly.
//       slice A 0x509529..0x509558 : fsub×2 + fabs×2 + fmul×2  (half-extents)
//       slice B 0x509598..0x50959e : fsub×1  (G44 − hh)
//       slice C 0x5095db..0x5095e1 : fsub×1  (G48 − hw)
//       slice D 0x509611..0x50963a : fsub×2 + fadd×1  (corner stores)
// ===========================================================================================

/// 0x509520 slice A (`0x509529`..`0x509558`): half-extents from the CRect `(l,t,r,b)` =
/// `([esi], [esi+4], [esi+8], [esi+0xc])`. `W = |r−l|` (`fld[esi+8];fsub[esi];fabs;fstp`);
/// `hh = |b−t|·0.5` (`fld[esi+0xc];fsub[esi+4];fabs;fmul[0.5];fstp`); `hw = W_f32·0.5`
/// (`fld[ebp-0x10];fmul[0.5];fstp` — reloads the f32-stored `W`). Returns `(W, hh, hw)` f32.
pub fn clamp_half_extents(l: f32, t: f32, r: f32, b: f32) -> (f32, f32, f32) {
    let w = (f64::from(r) - f64::from(l)).abs() as f32; // |r−l|
    let hh = ((f64::from(b) - f64::from(t)).abs() * f(HALF_BITS)) as f32; // |b−t|·0.5
    let hw = (f64::from(w) * f(HALF_BITS)) as f32; // W_f32·0.5
    (w, hh, hw)
}

/// 0x509520 single-op f32 subtract — slices B (`G44 − hh` @`0x509598`), C (`G48 − hw`
/// @`0x5095db`), and the D corner subs (`X − hh`, `Y − W`): `fld[m];fsub[m];fstp[m]`, one
/// PC_53 subtraction narrowed to f32 (exact: the difference of two f32s is f64-exact).
pub fn clamp_sub(a: f32, b: f32) -> f32 {
    (f64::from(a) - f64::from(b)) as f32
}

/// 0x509520 single-op f32 add — slice D corner `X + hh` (@`0x509634`): `fld[m];fadd[m];fstp[m]`.
pub fn clamp_add(a: f32, b: f32) -> f32 {
    (f64::from(a) + f64::from(b)) as f32
}
