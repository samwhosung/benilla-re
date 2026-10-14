//! Reimplements the arithmetic of the edit-box update and layout functions `0x77a790`,
//! `0x77a9f0`, `0x77ad10` and `0x77b8c0` (caret width, fill and text placement). The x87 runs at
//! 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests
//! in `tests/editbox_caret.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `[0x801624]` = 10.0 (`0x41200000`), the 0x77a9f0 height unit scale.
pub const TEN_BITS: u32 = 0x4120_0000;
/// `[0x801620]` = 0.1 (`0x3dcccccd`), the 0x77ad10 inset fraction.
pub const TENTH_BITS: u32 = 0x3dcc_cccd;
/// `[0x7ffd68]` = 1024.0 (`0x44800000`), the 0x77b8c0 blend numerator-scale.
pub const K1024_BITS: u32 = 0x4480_0000;
/// `[0x80306c]` = 4.0 (`0x40800000`), the 0x77b8c0 `fdivr` dividend.
pub const FOUR_BITS: u32 = 0x4080_0000;
/// `[0x832a4c]` = 1.0 (`0x3f800000`), the constant the `0x41ad70` helper pushes (st0 = 1.0).
pub const ONE_BITS: u32 = 0x3f80_0000;
/// `[0x832a44]` = 0.8 (`0x3f4ccccd`), the constant the `0x41ae60` degree-helper multiplies by.
pub const POINT8_BITS: u32 = 0x3f4c_cccd;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x77a790 — fill accumulate: [esi+0x374] += arg  (fadd ×1 @0x77a7dc).
// ===========================================================================================

/// `0x77a790`'s owned arithmetic (`0x77a7d9`..`0x77a7e2`): `fld [ebp+8]; fadd [esi+0x374];
/// fst [esi+0x374]`. The f32 argument `arg = [ebp+8]` is added to the running fill field
/// `field = [esi+0x374]` and stored back (`fst`, no pop — st0 is left live for the downstream
/// `fcomp [esi+0x370]`, but the OWNED observable is the f32 written to the field). Both operands
/// widen to f64; the sum is narrowed to f32 at the `fst dword`. Returns the stored f32.
pub fn fill_accumulate(field: f32, arg: f32) -> f32 {
    (f64::from(arg) + f64::from(field)) as f32
}

// ===========================================================================================
// (2) 0x77a9f0 — fetched height · 10.0  (fmul ×1 @0x77aa9d).
// ===========================================================================================

/// `0x77a9f0`'s owned arithmetic (`0x77aa9a`..`0x77aaa3`): `fld [ebp-4]; fmul [10.0]`. The f32
/// value fetched into `[ebp-4]` (the return of `0x7727b0`) is multiplied by `10.0` and `fstp`'d as
/// a layout-setter argument (the `fstp [esp]` after an interleaved object deref). `v` is the
/// fetched f32. Returns the f32 product.
pub fn height_x10(v: f32) -> f32 {
    (f64::from(v) * k(TEN_BITS)) as f32
}

// ===========================================================================================
// (3) 0x77ad10 — n·scale + scale·0.1, then two negated row offsets.
// ===========================================================================================

/// `0x77ad10`'s region-1 arithmetic (`0x77ada1`..`0x77adb3`): `fild qword [ebp-0xc]=n;
/// fmul [ebp+8]=scale; fld [ebp+8]; fmul [0.1]; faddp st(1),st; fstp [esp]`. Computes
/// `n·scale + scale·0.1` (the `faddp` adds the `0.1·scale` inset to the `n·scale` extent), all in
/// a single PC_53 f64 chain, narrowed to f32 at the `fstp`. `n` is the 64-bit integer `{count+1, 0}`
/// (`fild qword`); `scale` is the per-row height f32. Returns the f32 sum.
pub fn row_extent(n: i64, scale: f32) -> f32 {
    (n as f64 * f64::from(scale) + f64::from(scale) * k(TENTH_BITS)) as f32
}

/// `0x77ad10`'s region-2/region-3 arithmetic (`0x77ade8`..`0x77adf0` and `0x77ae28`..`0x77ae30`,
/// byte-identical): `fild qword [ebp-0xc]=n; fmul [ebp+8]=scale; fchs; fstp [esp]`. Computes
/// `-(n·scale)` — the negated Y offset of a row (`fchs` after the product), narrowed to f32 at the
/// `fstp`. `n` is the 64-bit integer row index (`{q, 0}` / `{q+1, 0}`); `scale` is the row height.
/// Returns the f32 negated offset.
pub fn row_offset_neg(n: i64, scale: f32) -> f32 {
    (-(n as f64 * f64::from(scale))) as f32
}

// ===========================================================================================
// (4) 0x77b8c0 — fill-edge resolves, a −anchor, and a constant blend.
// ===========================================================================================

/// `0x77b8c0`'s region-A/region-B size (`0x77b8df`..`0x77b902` and `0x77b923`..`0x77b93d`, same
/// shape): `fld trk_hi; fsub trk_lo; fdiv scale; fld anchor; fadd off; fsubp st(1),st; fstp [esp]`.
/// Computes `(trk_hi − trk_lo)/scale − (anchor + off)` — the box's span over its scale minus two text
/// insets, in a single PC_53 f64 chain narrowed to f32 at the `fstp`. Region A is the FontString's
/// width, `(RIGHT [0x70] − LEFT [0x68])/[0x7c] − (right [0x3f4] + left [0x3ec])`, set on every box;
/// region B is its height, `(TOP [0x6c] − BOTTOM [0x64])/[0x7c] − (top [0x3f0] + bottom [0x3e8])`, set
/// on a single-line box only (a multi-line box gets height 0 instead, `0x77b91c`). Returns the f32.
pub fn fill_edge(trk_hi: f32, trk_lo: f32, scale: f32, anchor: f32, off: f32) -> f32 {
    ((f64::from(trk_hi) - f64::from(trk_lo)) / f64::from(scale)
        - (f64::from(anchor) + f64::from(off))) as f32
}

/// `0x77b8c0`'s region-C negate (`0x77b943`..`0x77b956`): `fld [esi+0x3f0]; fchs; fstp [esp]`.
/// Computes `−top`, the `y` of the FontString's `SetPoint(TOPLEFT, box, TOPLEFT, left, −top)`
/// (`0x77b96b`), narrowed to f32 at the `fstp`. Returns the f32 negated inset.
pub fn neg_anchor(anchor: f32) -> f32 {
    (-f64::from(anchor)) as f32
}

/// `0x77b8c0`'s region-D constant blend (`0x77ba2d`..`0x77ba4d`): `call 0x41ad70` (pushes the
/// constant `[0x832a4c]=1.0`); `fmul [1024.0]; fdivr [4.0]; fstp [esp]; call 0x41ae60` (the degree
/// helper `[0x832a44]=0.8 · arg`); `fstp [ebp-4]`. This is the caret's stored width
/// `G1·4.0/(G3·1024.0)`. `G3 = [0x832a4c]` and `G1 = [0x832a44]` are runtime globals written
/// by `0x41ad10(aspect)`; `1.0`/`0.8` are their `.data` statics, the 4:3 values, so this function is
/// the 4:3 result: `inner = (4.0 / (1.0·1024.0))` narrowed to f32 (the `fstp [esp]`), then
/// `0.8·inner` narrowed again (the `fstp [ebp-4]`). Returns the f32.
pub fn blend_const() -> f32 {
    let inner = (k(FOUR_BITS) / (k(ONE_BITS) * k(K1024_BITS))) as f32; // fstp [esp]
    (k(POINT8_BITS) * f64::from(inner)) as f32 // 0x41ae60 reload + fstp [ebp-4]
}
