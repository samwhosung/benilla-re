//! Reimplements the arithmetic of the font loaders `0x77e8d0`, `0x77f0c0` and
//! `CFontObject::LoadXML` (`0x783c30`). The x87 runs at 53-bit precision, so each chain is `f64`
//! with `as f32` at the `fstp dword` stores; the tests in `tests/font_xml.rs` diff each against
//! the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact, cited by VA).
// ===========================================================================================

/// `[0x7ff9d8]` = 1.0 (`0x3f800000`), the reciprocal numerator of 0x77f0c0's `1.0 / scale`.
pub const ONE_BITS: u32 = 0x3f80_0000;
/// `[0x801628]` = 2.0 (`0x40000000`), the offset subtracted from each `delta·recip` product in
/// 0x77f0c0's Block 1 (`fsub dword [0x801628]`).
pub const TWO_BITS: u32 = 0x4000_0000;
/// `[0x7ffd68]` = 1024.0 (`0x44800000`), the gx-leaf-result multiplier in 0x783c30's spacing
/// unit-convert (`fmul dword [0x7ffd68]`).
pub const K1024_BITS: u32 = 0x4480_0000;
/// `[0x7ffd74]` = 0.0 (`0x00000000`), the clamp-low / divisor-zero compare operand in 0x77f0c0
/// (used in `fcomp` — a compare, NOT arithmetic; documented for context).
pub const ZERO_BITS: u32 = 0x0000_0000;
/// `[0x8029d4]` = 2^-22 ≈ 2.384e-7 (`0x34800000`), the magnitude-change epsilon in 0x783c30
/// (used in `fcomp` after each `fabs` — a compare, NOT arithmetic; documented for context).
pub const EPS_BITS: u32 = 0x3480_0000;

// ===========================================================================================
// (1) 0x77e8d0 — eight per-axis sign-flips of a struct f32 field.
// ===========================================================================================
//
// The body walks the six `[esi+0x3c]` edge-bit cases and the two prologue legs, and at each it
// loads a struct coordinate (`[esi+0x50]`, `[esi+0x5c]`, or `[esi+0x48]`), negates it, and stores
// the negated value as an `f32` argument for the layout setter `0x767c70` (orchestration past the
// owned `fchs`). The eight sites are byte-identical (`d9 e0`); each is the same owned op.

/// One `fchs` sign-flip (`fld dword [field]; fchs; fstp dword [m32]`): the struct coordinate `v`
/// (an `f32` widened to the PC_53 f64 chain) has its sign bit flipped, then stored back as `f32`.
/// On `+0.0`/`−0.0` the sign flips exactly as `fchs` does (`-0.0`/`+0.0`), so a zero input is NOT
/// idempotent — sign-of-zero discipline. `v` is the loaded field. Returns the stored f32.
pub fn negate(v: f32) -> f32 {
    (-f64::from(v)) as f32 // fld dword [field]; fchs; fstp dword [m32]  (PC_53 f64, narrowed on store)
}

// ===========================================================================================
// (2) 0x77f0c0 — reciprocal-scale × delta − 2.0 (×2), then clamped coordinate / divisor (×2).
// ===========================================================================================
//
// `esi = this` (`[esi+0x48]` = scale, `[esi+0x4c]` = an alternate scale); `edi = param_2` is a
// 4-`f32` rect `p[0..4]` (`*edi`, `[edi+4]`, `[edi+8]`, `[edi+0xc]`).
//
// Block 1 (`0x77f0fb`..`0x77f122`):
//   fld [0x7ff9d8]=1.0; fdiv [esi+0x48]        ; recip = 1.0 / scale     (st0, LIVE st(1) below)
//   fld [edi+0xc]; fsub [edi+4]                ; st0 = p[3]−p[1], st1 = recip
//   fmul st,st(1)                              ; st0 = (p[3]−p[1])·recip (LIVE recip on st(1))
//   fsub [0x801628]=2.0; fstp [ebp+8]          ; fVar2 = (p[3]−p[1])·recip − 2.0   (f32); pop
//   fld [edi+8]; fsub [edi]                    ; st0 = p[2]−p[0], st1 = recip
//   fmulp st(1),st                             ; st0 = (p[2]−p[0])·recip; pop
//   fsub [0x801628]=2.0; fst [ebp-4]           ; out10 = (p[2]−p[0])·recip − 2.0   (f32); NO pop
//   (the `fst` leaves st0 live ONLY for the two clamp `fcomp`s vs 0.0 — no further owned arith)
//
// Block 2 (`0x77f175`..`0x77f1e8`, guarded by `[esi+0x40] != 0` + the registered-glue check):
//   divisor4 = ([esi+0x4c] == 0.0) ? [esi+0x48] : [esi+0x4c]
//   fld [edi+0xc]; fsub [edi+4]; fdivrp st(1),st  ; A = (p[3]−p[1]) / divisor4   (SWAP: num/divisor)
//   divisor3 = ([esi+0x4c] == 0.0) ? [esi+0x48] : [esi+0x4c]   (same select)
//   fld [edi+8]; fsub [edi]; fdivrp st(1),st      ; B = (p[2]−p[0]) / divisor3
//   (A → out4/local_30[4]; B → out3/local_30[3]; both stored as f32 for the setter)

/// 0x77f0c0 Block 1 — the two `delta·recip − 2.0` products. `scale` is `[esi+0x48]`; `p0..p3` are
/// the rect floats `*edi`..`[edi+0xc]`. Returns `(fVar2, out10)` = the two stored f32 results:
/// `fVar2 = (p3−p1)·(1/scale) − 2.0` (the LIVE-recip `fmul st,st(1)` product), and
/// `out10 = (p2−p0)·(1/scale) − 2.0` (the `fmulp` product). Both keep the f64 reciprocal un-rounded
/// across the multiply (live on st(1)); each narrows to f32 only at its `fstp`/`fst dword`.
pub fn block1_offsets(scale: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> (f32, f32) {
    let recip = k(ONE_BITS) / f64::from(scale); // fld 1.0; fdiv [esi+0x48]   (f64, LIVE st(1))
    let two = k(TWO_BITS);
    let d_hi = f64::from(p3) - f64::from(p1); // fld [edi+0xc]; fsub [edi+4]
    let f_var2 = (d_hi * recip - two) as f32; // fmul st,st(1); fsub [0x801628]; fstp [ebp+8] (f32)
    let d_lo = f64::from(p2) - f64::from(p0); // fld [edi+8]; fsub [edi]
    let out10 = (d_lo * recip - two) as f32; // fmulp st(1),st; fsub [0x801628]; fst [ebp-4] (f32)
    (f_var2, out10)
}

/// 0x77f0c0 Block 2's divisor selection: `([esi+0x4c] == 0.0) ? [esi+0x48] : [esi+0x4c]`. The
/// source uses `fld [esi+0x4c]; fcomp [0.0]; test ah,0x44; jnp` — on EQUAL (incl. ±0) it takes the
/// `[esi+0x48]` leg. `alt` is `[esi+0x4c]`, `scale` is `[esi+0x48]`. Returns the chosen divisor f32.
pub fn block2_divisor(alt: f32, scale: f32) -> f32 {
    if alt == 0.0 {
        scale
    } else {
        alt
    }
}

/// 0x77f0c0 Block 2 — the two clamped `delta / divisor` quotients via `fdivrp` SWAP. `alt`
/// (`[esi+0x4c]`) and `scale` (`[esi+0x48]`) drive the per-quotient divisor selection; `p0..p3` are
/// the rect floats. Returns `(out4, out3)` = the two stored f32 results: `out4 = (p3−p1)/divisor`,
/// `out3 = (p2−p0)/divisor`, where `divisor = block2_divisor(alt, scale)` (same select each time).
/// The `fdivrp st(1),st` computes `numerator / divisor` (the numerator is loaded LAST onto st0).
pub fn block2_quotients(alt: f32, scale: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> (f32, f32) {
    let div4 = f64::from(block2_divisor(alt, scale));
    let a = ((f64::from(p3) - f64::from(p1)) / div4) as f32; // fld [edi+0xc]; fsub [edi+4]; fdivrp
    let div3 = f64::from(block2_divisor(alt, scale));
    let b = ((f64::from(p2) - f64::from(p0)) / div3) as f32; // fld [edi+8]; fsub [edi]; fdivrp
    (a, b)
}

// ===========================================================================================
// (3) 0x783c30 — font-height epsilon test + spacing unit-convert epsilon test.
// ===========================================================================================
//
// Region A (`0x783e45`..`0x783e4d`): `fld [esi+0x48]; fsubr [ebp-4]; fabs; fcomp [eps]`
//   The current font height `cur` (`[esi+0x48]` = param_1[0x12]) is reverse-subtracted from the new
//   height `new` (`[ebp-4]`): `fsubr [m32]` = `[m32] − st0` = new − cur; `fabs` → |new − cur|. The
//   `fcomp [0x8029d4]` (eps) is a compare (excluded) — only the `|·|` is owned arith.
//
// Region B spacing (`0x783ef6`..`0x783f1c`): two foreign gx-leaf calls bracket the owned arith:
//   call 0x64aaa0 (atof) -> st0 = spacing;  fstp [ebp+8]  (DELEGATED leaf; store spacing f32)
//   call 0x41ad70        -> st0 = scaleFactor             (DELEGATED gx leaf)
//   fmul [0x7ffd68]=1024.0                                ; st0 = scaleFactor · 1024.0   (OWNED)
//   fdivr [ebp+8]                                         ; st0 = spacing / (scaleFactor·1024) (OWNED SWAP)
//   fstp [esp]; call 0x41ae60 -> st0 = newSpacing         (DELEGATED gx leaf consumes the quotient)
//   fld [esi+0x50]; fsubr st,st(1)                        ; st0 = newSpacing − oldSpacing  (OWNED)
//   fabs; fcomp [eps]                                     ; |Δ|, compare (the fabs is OWNED)

/// 0x783c30 Region A's owned `fsubr; fabs`: `|new − cur|` (`fld [esi+0x48]=cur; fsubr [ebp-4]=new;
/// fabs`). The `fsubr dword [m32]` reverse-subtracts (`new − cur`, NOT cur − new). `cur` is the
/// current font height, `new` the candidate. Returns the magnitude `|new − cur|` (the value the
/// source `fabs` leaves on st0 before the epsilon `fcomp`).
pub fn height_delta_abs(cur: f32, new: f32) -> f64 {
    (f64::from(new) - f64::from(cur)).abs() // fld [esi+0x48]; fsubr [ebp-4]; fabs  (PC_53 f64)
}

/// 0x783c30 Region B's owned `fmul; fdivr`: `spacing / (scale_factor · 1024.0)` (`fmul [0x7ffd68];
/// fdivr [ebp+8]`). The `fmul dword [0x7ffd68]` multiplies the gx-leaf scale factor by 1024.0; the
/// `fdivr dword [ebp+8]` reverse-divides (`spacing / product`, the memory operand `spacing` is the
/// DIVIDEND). `scale_factor` is the `0x41ad70` leaf result; `spacing` the atof result. Returns the
/// f64 quotient the source `fstp [esp]` then narrows to f32 for the gx leaf `0x41ae60` (delegated).
pub fn spacing_quotient(scale_factor: f32, spacing: f32) -> f64 {
    let product = f64::from(scale_factor) * k(K1024_BITS); // fmul dword [0x7ffd68]=1024.0
    f64::from(spacing) / product // fdivr dword [ebp+8]  (SWAP: spacing / product)
}

/// The f32 the source's `fstp [esp]` actually hands the gx leaf — [`spacing_quotient`] narrowed to
/// `f32` (PC_53 f64 product, `as f32` at the `fstp dword [esp]`). Returns the stored f32 arg.
pub fn spacing_quotient_f32(scale_factor: f32, spacing: f32) -> f32 {
    spacing_quotient(scale_factor, spacing) as f32
}

/// 0x783c30 Region B's owned `fsubr st,st(1); fabs`: `|new_spacing − old_spacing|`. The
/// `fsubr st,st(1)` computes `st(1) − st0` = `new_spacing − old_spacing` (old on st0 from
/// `fld [esi+0x50]`, new on st(1) from the gx leaf). `old` is `[esi+0x50]`, `new` the leaf result.
/// Returns the magnitude (the value before the epsilon `fcomp`).
pub fn spacing_delta_abs(old: f32, new: f32) -> f64 {
    (f64::from(new) - f64::from(old)).abs() // fld [esi+0x50]; fsubr st,st(1); fabs  (PC_53 f64)
}
