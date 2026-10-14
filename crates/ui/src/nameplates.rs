//! Reimplements the arithmetic of the name-plate functions `CGNamePlateFrame::Bind` (`0x7cb6d0`),
//! `OnUpdate` (`0x7cb950`), `DisplayedLevel` (`0x7cbce0`) and `SetRaidIcon` (`0x7cbe20`). The x87
//! runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the
//! tests in `tests/nameplates.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_0081df00` = 0.031 (`0x3cfdf3b6`), 0x7cb6d0 scale #1 (X).
pub const K0_BITS: u32 = 0x3cfd_f3b6;
/// `DAT_00802978` = 0.125 (`0x3e000000`), 0x7cb6d0 scale #2 (Y).
pub const K1_BITS: u32 = 0x3e00_0000;
/// `DAT_0081defc` = 0.804 (`0x3f4dd2f2`), 0x7cb6d0 scale #3 (X).
pub const K2_BITS: u32 = 0x3f4d_d2f2;
/// `DAT_0081def8` = 0.281 (`0x3e8fdf3b`), 0x7cb6d0 scale #4 (Y).
pub const K3_BITS: u32 = 0x3e8f_df3b;
/// `DAT_0081def4` = −0.092 (`0xbdbc6a7f`), 0x7cb6d0 scale #5 (X).
pub const K4_BITS: u32 = 0xbdbc_6a7f;
/// `DAT_0081def0` = 0.284 (`0x3e916873`), 0x7cb6d0 scale #6 (Y).
pub const K5_BITS: u32 = 0x3e91_6873;
/// `DAT_008022cc` = 1000.0 (`0x447a0000`), the 0x7cb950 unit scale.
pub const SCALE1000_BITS: u32 = 0x447a_0000;
/// `DAT_008029c8` = 0.05 (`0x3d4ccccd`), the 0x7cbce0 percent step.
pub const PCT05_BITS: u32 = 0x3d4c_cccd;
/// `DAT_007ffa24` = 0.5 (`0x3f000000`), the 0x7cbce0 round-to-nearest bias.
pub const HALF_BITS: u32 = 0x3f00_0000;
/// `DAT_008029b0` = 0.25 (`0x3e800000`), the 0x7cbe20 quarter step + bias.
pub const QUARTER_BITS: u32 = 0x3e80_0000;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x7cb6d0 — 6× independent `global · const → f32` scale stores.
// ===========================================================================================

/// The single `fld [G]; fmul [k]; fstp [m32]` scale shape shared by all six of `0x7cb6d0`'s owned
/// products (`0x7cb74d`/`761`/`78e`/`7ac`/`7c7`/`7d9`). Each loads a runtime coordinate global
/// (`[0x87d9cc]` = X, `[0x87d9d0]` = Y), multiplies by a per-store constant, and `fstp`s the result
/// → f32 (then pushes it as a layout-setter `0x767c70` argument). `g` is the global, `kbits` its
/// constant. Returns the f32 product.
pub fn scale(g: f32, kbits: u32) -> f32 {
    (f64::from(g) * k(kbits)) as f32
}

/// The full 6-store sequence of `0x7cb6d0`, exactly as the disasm orders them. `gx = [0x87d9cc]`,
/// `gy = [0x87d9d0]` are the two coordinate globals; the six stores are
/// `(gx·K0, gy·K1, gx·K2, gy·K3, gx·K4, gy·K5)`. Each is an independent product (no register
/// carry between them — each is `fld;fmul;fstp`), so order is irrelevant to the values, but the
/// constants pair X/Y/X/Y/X/Y across the six. Returns the six f32 products.
pub fn scale6(gx: f32, gy: f32) -> [f32; 6] {
    [
        scale(gx, K0_BITS),
        scale(gy, K1_BITS),
        scale(gx, K2_BITS),
        scale(gy, K3_BITS),
        scale(gx, K4_BITS),
        scale(gy, K5_BITS),
    ]
}

// ===========================================================================================
// (2) 0x7cb950 — arg · 1000.0 then `__ftol` (truncate toward zero).
// ===========================================================================================

/// `0x7cb950`'s owned arithmetic (`0x7cb962`..`0x7cb96b`): `fld [ebp+8]; fmul [1000.0]; __ftol`.
/// The product `arg · 1000.0` (a PC_53 f64) is converted to i32 by the CRT `__ftol` (`0x40a2b0`),
/// which **truncates toward zero**. The result is subtracted from a frame counter `[+0x514]`
/// downstream (orchestration past this slice). `arg = [ebp+8]`. Returns the truncated i32.
pub fn scale1000_ftol(arg: f32) -> i32 {
    let v = f64::from(arg) * k(SCALE1000_BITS);
    benilla_re_cmath::x87::ftol(v) // __ftol = truncate toward zero
}

// ===========================================================================================
// (3) 0x7cbce0 — round-to-nearest of n·0.05 via +0.5 bias then `__ftol`.
// ===========================================================================================

/// `0x7cbce0`'s clamp+round (`0x7cbd1b`..`0x7cbd36`): the input byte `b = [+0x1d]` is clamped to
/// `min(b, 100)` (an unsigned-byte `cmp al,0x64; jb; mov al,0x64`), zero-extended to i32, then
/// `fild [n]; fmul [0.05]; fadd [0.5]; __ftol`. So `out = (i32)(n·0.05 + 0.5)` truncate =
/// round-half-up of `n·0.05` (n is a non-negative 0..=100, so the `+0.5` bias gives
/// round-to-nearest). The result is subtracted from `[+0x70]` downstream. `b` is the raw byte
/// (0..=255 as the caller passes it). Returns the truncated i32.
pub fn percent_round(b: u8) -> i32 {
    let n = u32::from(b.min(100)); // cmp al,0x64; jb; mov al,0x64; movzx
    let v = (f64::from(n) * k(PCT05_BITS)) + k(HALF_BITS);
    benilla_re_cmath::x87::ftol(v) // __ftol = truncate toward zero
}

// ===========================================================================================
// (4) 0x7cbe20 — split count → 2 quarter-scaled pairs (val, val+0.25).
// ===========================================================================================

/// `0x7cbe20`'s quarter-split (`0x7cbe4d`..`0x7cbe98`, the `count >= 8` path). The input count
/// `c = [ebp+8]` is split into `lo = c & 3` (0..=3) and `hi = c >> 2`, each `fild`'d and scaled by
/// `0.25`, and a `(value, value+0.25)` pair is produced for each — the four f32 stores feed a
/// texture/UV rect builder (`0x770410`):
/// * `a = (c & 3)·0.25`      → `[ebp-0xc]`  (`fmul [0.25]`)
/// * `b = (c >> 2)·0.25`     → `[ebp-0x10]` (`fmul [0.25]`, `fst` no-pop → live)
/// * `c_out = a + 0.25`      → `[ebp-0x4]`  (`fadd [0.25]` on the LIVE PC_53 `a`)
/// * `d = b + 0.25`          → `[ebp-0x8]`  (`fadd [0.25]` on the LIVE PC_53 `b`)
///
/// The disasm order is `a·` then `b·`, then (via `fld st(1)`/`fxch`) `a` is biased to `c_out`
/// while `b` stays live for `d`. Returns `(a, b, c_out, d)` = `[ebp-0xc]`/`[ebp-0x10]`/`[ebp-0x4]`/
/// `[ebp-0x8]` as f32. `count` is `[ebp+8]` (the caller guarantees `>= 8` for this path).
pub fn quarter_split(count: u32) -> (f32, f32, f32, f32) {
    let lo = f64::from(count & 3); // fild {c&3, 0}
    let hi = f64::from(count >> 2); // fild {c>>2, 0}
    let q = k(QUARTER_BITS); // 0.25
    let a = lo * q; // st0 = a, LIVE
    let b = hi * q; // st0 = b, LIVE
    let c_out = a + q; // fadd [0.25] on live a
    let d = b + q; // fadd [0.25] on live b
    (a as f32, b as f32, c_out as f32, d as f32)
}
