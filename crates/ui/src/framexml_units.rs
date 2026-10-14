//! Reimplements the arithmetic of the unit-conversion and colour functions `0x788750`,
//! `0x78ab40`, `0x78bae0` and `0x78bcf0`. The x87 runs at 53-bit precision, so each chain is
//! `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/framexml_units.rs` diff
//! each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact; the difftest asserts these against the mapped
// image).
// ===========================================================================================

/// `[0x804578]` (`.rdata`) = the exact `f64` `1.0/255.0` (`0x3f70101010101010`), 0x78bae0's
/// per-channel color normalize multiplier (`fmul qword [0x804578]`). (Same f64 `1/255` as
/// `luabind_76e`'s `INV255_F64_BITS`.)
pub const INV255_F64_BITS: u64 = 0x3f70_1010_1010_1010;
/// `[0x7ffd68]` (`.rdata`) = `1024.0` as a **dword** `f32` (`0x44800000`), 0x78ab40's pixels→units
/// multiplier (`fmul dword [0x7ffd68]`). Widened to f64 in the multiply.
pub const PX1024_F32_BITS: u32 = 0x4480_0000;

#[inline]
fn inv255_f64() -> f64 {
    f64::from_bits(INV255_F64_BITS)
}
#[inline]
fn px1024_f32() -> f64 {
    f64::from(f32::from_bits(PX1024_F32_BITS)) // f32 operand widened exactly to f64
}

// ===========================================================================================
// (1) 0x788750 — two `(hi − lo) / cell` ratios, a leaf-size sum, and a per-iteration subtract.
// ===========================================================================================
//
// `esi = this`. `[esi+0x374]/[esi+0x36c]` (and `[esi+0x378]/[esi+0x370]`) are coordinate bounds;
// `[esi+0x7c]` is a per-cell size. `[ebp+8]` (at the loop body) holds the f32 leaf-size the function
// stored via `fstp [ebp+8]` from `0x766750`/`0x7729b0`.
//   fld [esi+0x374]; fsub [esi+0x36c]; fdiv [esi+0x7c]; fstp [ebp-0x14]   ; local_18 = (.374−.36c)/.7c
//   fld [esi+0x378]; fsub [esi+0x370]; fdiv [esi+0x7c]; fstp [esp]        ; arg     = (.378−.370)/.7c
//   <0x7729b0()=st0>; fadd [ebp+8]; fstp [ebp+8]                          ; total  = size7729b0 + size766750
//   fld [ebp-0x14]; fsub [ebp+8]; fstp [ebp-0x14]                         ; local_18 -= total  (each iter)

/// One `(hi − lo) / cell` ratio of 0x788750 (`fld [hi]; fsub [lo]; fdiv [cell]; fstp [m32]`). All
/// three operands are `f32` memory, widened to f64; the result narrows to f32 at the `fstp dword`.
/// `hi`/`lo` are the coordinate bounds, `cell` the per-cell size. Returns the stored f32 ratio.
/// (Used for both the `[esi+0x374]/[esi+0x36c]` and `[esi+0x378]/[esi+0x370]` blocks — same shape.)
pub fn bounds_ratio(hi: f32, lo: f32, cell: f32) -> f32 {
    ((f64::from(hi) - f64::from(lo)) / f64::from(cell)) as f32
}

/// 0x788750's leaf-size sum (`<0x7729b0()=st0>; fadd dword [ebp+8]; fstp dword [ebp+8]`): the
/// `0x7729b0` leaf result on `st0` is added to the f32 `0x766750` leaf result the function stored at
/// `[ebp+8]`, and stored back as f32. `a` is the `st0` value (the `0x7729b0` result, an `f32`-typed
/// leaf return), `b` the f32 already at `[ebp+8]`. Returns the stored f32 sum.
pub fn size_sum(a: f32, b: f32) -> f32 {
    (f64::from(a) + f64::from(b)) as f32
}

/// 0x788750's per-iteration subtract (`fld dword [ebp-0x14]; fsub dword [ebp+8]; fstp dword
/// [ebp-0x14]`): the running `local_18` f32 at `[ebp-0x14]` minus the f32 `total` at `[ebp+8]`,
/// stored back as f32. `acc` is `[ebp-0x14]`, `total` the `[ebp+8]` value. Returns the stored f32.
pub fn acc_subtract(acc: f32, total: f32) -> f32 {
    (f64::from(acc) - f64::from(total)) as f32
}

// ===========================================================================================
// (2) 0x78ab40 — width/height unit-convert (`atof / (px · 1024)`) + a size subtract.
// ===========================================================================================
//
// Per dimension: `0x64aaa0` (atof) leaves the parsed value on `st0`, `fstp`'d to f32 at the cell
// (`[ebp-0x10]` width / `[ebp-0x14]` height). `0x41ad70` (a pixel-count leaf) leaves a value on st0,
// `fmul [0x7ffd68]=1024.0`, then `fdivr [cell]` = `cell / (px·1024)`, `fstp`'d f32 to `[esp]` and
// handed to `0x41ae60`. Numerically `atof / (px · 1024)`.
//   call atof; fstp [ebp-0x10]
//   call px_leaf; fmul [0x7ffd68]; fdivr [ebp-0x10]; fstp [esp]; call 0x41ae60; fstp [ebp-0x10]
// The size subtract (`!align` path): a `+0x20` vtable getter leaves st0; `fsubr [edi+0x34c]` =
// `[edi+0x34c] − st0`; `fstp [edi+0x34c]`.

/// 0x78ab40's owned unit-convert arithmetic (`fmul dword [0x7ffd68]=1024.0; fdivr dword [cell]`):
/// the `0x41ad70` pixel-count leaf result `px` (on `st0`, a PC_53 f64) is `· 1024.0`, then the
/// stored atof'd dimension `dim` (the f32 at `[ebp-0x10]`/`[ebp-0x14]`) is divided BY it — `fdivr`
/// SWAPS, so the result is `dim / (px·1024)`, NOT `(px·1024)/dim`. The `fstp dword [esp]` narrows to
/// f32 (then handed to the `0x41ae60` leaf — not owned). `px` is the `st0` leaf result, `dim` the
/// f32 dimension. Returns the f32 stored at `[esp]`.
pub fn unit_convert(px: f32, dim: f32) -> f32 {
    let denom = f64::from(px) * px1024_f32(); // fmul dword [0x7ffd68]
    (f64::from(dim) / denom) as f32 // fdivr dword [cell]  (cell / denom — SWAP)
}

/// 0x78ab40's owned size subtract (`fsubr dword [edi+0x34c]; fstp dword [edi+0x34c]`): the `+0x20`
/// vtable getter result `s` (on `st0`) is subtracted FROM the field `[edi+0x34c]` — `fsubr` SWAPS,
/// so the result is `field − s`, NOT `s − field`. Stored back as f32. `field` is `[edi+0x34c]`, `s`
/// the `st0` getter result. Returns the stored f32.
pub fn size_subtract(field: f32, s: f32) -> f32 {
    (f64::from(field) - f64::from(s)) as f32
}

// ===========================================================================================
// (3) 0x78bae0 — three packed-color channel normalizes byte·(1/255 f64), pushed as f64.
// ===========================================================================================
//
// A packed color (`[ebp-0x4]`, low dword from `0x7bbec0`) is split into three channel bytes
// `(al, ah, [ebp-0x2])`, each `fild dword [ebp-0x8]` (zero-extended into a dword), `fmul qword
// [0x804578]=1/255(f64)`, `fstp qword [esp+..]` — keeping the f64 product (no f32 narrow), straight
// into the `0x7026f0` `"%f %f %f"` marshaller. Push order: `al` (→[esp+0x10]), `ah` (→[esp+8]),
// `[ebp-0x2]` (→[esp]).

/// One 0x78bae0 channel normalize (`fild dword [byte]; fmul qword [0x804578]; fstp qword`): the
/// channel byte (`0..=255`, zero-extended into a dword and `fild`'d — exact) `· (1/255 f64)`, kept
/// as the **f64** product (pushed as a `%f` double arg, NOT narrowed). `byte` is the channel byte.
/// Returns the f64 product. (Same shape as `luabind_76e::alpha_channel`, but reached via `fild dword`
/// of a `movzx`'d byte.)
pub fn color_channel(byte: u8) -> f64 {
    f64::from(i32::from(byte)) * inv255_f64() // fild dword (exact); fmul qword [1/255 f64]; fstp qword
}

/// 0x78bae0 — the three channel normalizes in push order. `c_lo` is `al` (low byte of `[ebp-0x4]`,
/// pushed to `[esp+0x10]`), `c_mid` is `ah` (pushed to `[esp+8]`), `c_hi` is `[ebp-0x2]` (byte 2 of
/// `[ebp-0x4]`, pushed to `[esp]`). Returns the three f64 products in push order `(c_lo, c_mid,
/// c_hi)`.
pub fn color_three(c_lo: u8, c_mid: u8, c_hi: u8) -> (f64, f64, f64) {
    (
        color_channel(c_lo),
        color_channel(c_mid),
        color_channel(c_hi),
    )
}

// ===========================================================================================
// (4) 0x78bcf0 — one scaled-height product.
// ===========================================================================================
//
// `0x78bcf0` is `CSimpleColorSelect`'s **value-thumb positioner**: the `+0x20` geometry-vtable slot
// is `GetHeight`, called on `[esi+0x318]` — the colour wheel's region, not the value strip's — and
// `[esi+0x330]` is the widget's HSV **value** `V`. The product is the thumb's Y offset above the
// strip's bottom edge, handed to `SetPoint 0x767c70(CENTER, strip.geometry, BOTTOM, 0, y, 1)` (not
// owned). Neither operand is an alpha.
//   call GetHeight(wheel.geometry); fmul [esi+0x330] (=V); fstp [ebp-0x4]

/// 0x78bcf0's owned product (`<GetHeight()=st0>; fmul dword [esi+0x330]; fstp dword [ebp-0x4]`):
/// the `+0x20` geometry-vtable `GetHeight` result `a` (on `st0`, taken off the COLOUR WHEEL region
/// `[esi+0x318]`) `· [esi+0x330]` (the widget's HSV value `V`), narrowed to f32 at the
/// `fstp dword`. Returns the stored f32 — the value thumb's Y offset above the strip's bottom edge.
pub fn thumb_height_product(a: f32, scale: f32) -> f32 {
    (f64::from(a) * f64::from(scale)) as f32 // fmul dword [esi+0x330]
}
