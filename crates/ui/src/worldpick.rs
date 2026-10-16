//! Reimplements the arithmetic inside the pick and chat-colour functions: the distance and
//! normalise in the pick tracer `TracePick` (`0x480df0`) and its driver `0x480a50`, the
//! milliseconds-to-minutes product of the play-time poll in `0x495590`, and the byte-to-unit colour channels of `LoadChatSettings` (`0x498a60`).
//! The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword`
//! stores; the tests in `tests/worldpick.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact; the difftest asserts these against the mapped
// image at their VAs).
// ===========================================================================================

/// `[0x8026c8]` (`.rdata`) = `1.0/255.0` as a **dword** `f32` (`0x3b808081`), 0x498a60's per-channel
/// color-normalize multiplier (`fmul dword [0x8026c8]`). (Same f32 `1/255` as `luabind_76e`'s
/// `INV255_F32_BITS`.) Widened to f64 in the multiply.
pub const INV255_F32_BITS: u32 = 0x3b80_8081;
/// `[0x804590]` (`.rdata`) = `1.6666666851961054e-05` as a **dword** `f32` (`0x378bcf65`), 0x495590's
/// `(i64)left · k` minutes multiplier (`fmul dword [0x804590]`; numerically ≈ `1/60000`). Widened to f64.
pub const K_804590_F32_BITS: u32 = 0x378b_cf65;
/// `[0x8029d4]` (`.rdata`) = `2.384185791015625e-07` (`0x34800000`), 0x480df0's `fabs(dist)`
/// vs-epsilon compare operand (a `fcomp` compare, NOT arithmetic; documented for context).
pub const DIST_EPS_F32_BITS: u32 = 0x3480_0000;

#[inline]
fn inv255_f32() -> f64 {
    f64::from(f32::from_bits(INV255_F32_BITS)) // f32 operand widened exactly to f64
}
#[inline]
fn k_804590() -> f64 {
    f64::from(f32::from_bits(K_804590_F32_BITS))
}

// ===========================================================================================
// (1) 0x480a50 — coordinate deltas + a LIVE-stack sum-of-squares · scale.
// ===========================================================================================
//
// The two delta vectors `param_2 − c` and `param_3 − c` (each `fld dword; fsub dword; fstp dword`,
// stored into the `0x4549a0` arg buffer) carry six owned `fsub`s. Then the owned distance compute
// (`0x480c46`..`0x480c81`):
//   fld d0h; fsub d0l; fld d1h; fsub d1l; fld d2h; fsub d2l   ; st0=d2 st1=d1 st2=d0   (3 fsub)
//   fld st(0); fmul st,st(1)                                  ; st0=d2²              (1 fmul)
//   fld st(2); fmul st,st(3); faddp st(1),st                  ; st0=d1²+d2²    (1 fmul,1 faddp)
//   fld st(3); fmul st,st(4); faddp st(1),st                  ; st0=d0²+d1²+d2² (1 fmul,1 faddp)
//   fsqrt                                                     ; st0=|v|          (1 fsqrt)
//   fstp st(3); fstp st(0); fstp st(0)                        ; clear the deltas, st0=|v|
//   fmul dword [scale]                                        ; st0=|v|·scale    (1 fmul)
//   fst dword [out0]; fstp dword [out1]                       ; store the SAME f64 to both, as f32

/// One coordinate delta of 0x480a50: `(a − c) as f32` (`fld dword [a]; fsub dword [c]; fstp dword`).
/// Both operands are `f32` memory widened to f64; the result narrows to f32 at the `fstp dword`.
/// `a`/`c` are the struct coordinate / centre. Returns the stored f32 delta. (Used for all six
/// `param_2/param_3 − centre` components.)
pub fn coord_delta(a: f32, c: f32) -> f32 {
    (f64::from(a) - f64::from(c)) as f32
}

/// 0x480a50's owned distance product (`0x480c46`..`0x480c7b`): three deltas `d0,d1,d2` (each
/// `(hi − lo)` of two `f32` locals, exact f64), squared and summed entirely on the x87 stack as f64
/// (`fmul st,st(k)` — NO intervening f32 round), `fsqrt`'d, then `· scale` (the `[ebp-0x8]` f32
/// widened). The product is the f64 value the final `fst dword [ecx+0xc]` / `fstp dword [edx]`
/// narrow to f32 (the same f64 stored to both slots). `(h0,l0)`/`(h1,l1)`/`(h2,l2)` are the three
/// delta operand pairs (f32 locals); `scale` is the `[ebp-0x8]` f32. Returns the f32 stored.
///
/// The summation order matches the source stack exactly: `st0=d2` is squared first, then `d1²` is
/// `faddp`'d, then `d0²` — i.e. `(d2² + d1²) + d0²`.
pub fn dist_scale(h0: f32, l0: f32, h1: f32, l1: f32, h2: f32, l2: f32, scale: f32) -> f32 {
    let d0 = f64::from(h0) - f64::from(l0); // fld d0h; fsub d0l   (exact f64 delta)
    let d1 = f64::from(h1) - f64::from(l1);
    let d2 = f64::from(h2) - f64::from(l2);
    let sumsq = (d2 * d2 + d1 * d1) + d0 * d0; // squares/sum kept LIVE f64 (fmul st,st(k); faddp)
    (sumsq.sqrt() * f64::from(scale)) as f32 // fsqrt; fmul dword [scale]; fst/fstp dword
}

// ===========================================================================================
// (2) 0x480df0 — chord-length sqrt·scale (block 0) + vector normalize (blocks A/B, same arith).
// ===========================================================================================
//
// Block 0 (`0x480e4a`..`0x480e75`) is the same LIVE-stack sum-of-squares · scale as 0x480a50, but
// with the three deltas read directly from `[esi]/[edi]` (the two point structs) — covered by
// [`dist_scale`] (identical opcode shape). Its result is `[ebp-0x4]`'s scale (local_8).
//
// Blocks A (`0x480ec5`..`0x480f47`) and B (`0x480f8b`..`0x48100c`) are the vector normalize, reached
// after the delegated `0x4549a0` builds the local delta vector and `0x4549f0` leaves dist² on st0:
//   fsqrt                            ; st0 = dist
//   fld st(0); fabs; fcomp [eps]; (branch)   ; guard |dist| vs eps — fabs owned, fcomp/branch excluded
//   fdivr dword [ebp+0xc]            ; st0 = scalar / dist   (SWAP — mem/st0)        = ratio (LIVE)
//   fld [ebp-0xc];  fmul st,st(1); fstp dword [esp+8]   ; vy·ratio → f32   (ratio LIVE on st1)
//   fld [ebp-0x10]; fmul st,st(1); fstp dword [esp+4]   ; vx·ratio → f32
//   fld [ebp-0x14]; fmul st,st(1); fstp dword [esp]     ; vz·ratio → f32
//   fstp st(0)                      ; discard the ratio

/// 0x480df0's `fabs(dist)` magnitude (the `fld st(0); fabs` before the near-zero `fcomp [0x8029d4]`
/// guard): the post-`fsqrt` distance has its **magnitude** taken (`fabs`, the sole arithmetic op of the
/// guard — the `fcomp` vs the epsilon is a compare, excluded). `dist` is the `sqrt` result. Returns
/// `|dist|`.
pub fn dist_abs(dist: f64) -> f64 {
    dist.abs() // fld st(0); fabs  (PC_53 f64)
}

/// 0x480df0's normalize reciprocal-ratio (`fsqrt; … fdivr dword [ebp+0xc]`): `dist = sqrt(dist²)`
/// (the `0x4549f0` result on st0), then `ratio = scalar / dist` — the `fdivr` SWAPS, dividing the
/// `[ebp+0xc]` scalar BY the distance (NOT distance by scalar). Kept f64, LIVE for the three
/// component muls. `dist2` is the `0x4549f0` result (st0 before the `fsqrt`); `scalar` is `[ebp+0xc]`
/// (an f32 widened). Returns the f64 ratio.
pub fn norm_ratio(dist2: f64, scalar: f32) -> f64 {
    let dist = dist2.sqrt(); // fsqrt
    f64::from(scalar) / dist // fdivr dword [ebp+0xc]  (scalar / dist — SWAP)
}

/// One normalized component of 0x480df0's normalize blocks A/B: `(comp · ratio) as f32` where
/// `ratio = scalar / dist` is the LIVE st1 reused for all three components WITHOUT an intervening
/// f32 round. `comp` is the `[ebp-0xc]`/`[ebp-0x10]`/`[ebp-0x14]` f32 vector component (widened to
/// f64); `ratio` is the live f64 [`norm_ratio`]. Returns the f32 stored. (`fld dword [comp];
/// fmul st,st(1); fstp dword`.)
pub fn norm_component(comp: f32, ratio: f64) -> f32 {
    (f64::from(comp) * ratio) as f32 // fmul st,st(1) on the live ratio
}

/// 0x480df0's full normalize block (A and B are the same arithmetic): from the `0x4549f0` dist²
/// (`dist2`), the `[ebp+0xc]` scalar, and the three local vector components `(cy, cx, cz)` (the
/// `[ebp-0xc]`/`[ebp-0x10]`/`[ebp-0x14]` order the source multiplies), produce the three normalized
/// f32 stores in push order `([esp+8], [esp+4], [esp])`. Returns `(out0, out1, out2)`.
pub fn normalize_block(dist2: f64, scalar: f32, cy: f32, cx: f32, cz: f32) -> (f32, f32, f32) {
    let ratio = norm_ratio(dist2, scalar);
    (
        norm_component(cy, ratio),
        norm_component(cx, ratio),
        norm_component(cz, ratio),
    )
}

// ===========================================================================================
// (3) 0x495590 — the play-time poll's `(i64)left · k` product, kept f64.
// ===========================================================================================
//
//   fild qword [ebp-0x8]; fmul dword [0x804590]; fstp qword [esp]
// The signed 64-bit integer at `[ebp-0x8]` (low dword) / `[ebp-0x4]` (high dword) is `fild`'d (exact
// for values in i53), `· k` (the f32 `0x804590` widened), kept f64 (pushed to `0x73fdf5`, then the
// CRT `__ftol` `0x40a2b0` truncates; `billing::minutes_left` covers those).

/// 0x495590's owned product (`fild qword; fmul dword [0x804590]; fstp qword`): the signed 64-bit
/// integer `n` (the milliseconds left) `· k` (the f32 `0x804590` ≈ 1/60000, widened to f64), kept as
/// the un-narrowed PC_53 f64 (pushed to `ceil`). `n` is the `[ebp-0x8]` qword. Returns the f64
/// product.
pub fn tick_scale(n: i64) -> f64 {
    n as f64 * k_804590() // fild qword (exact in i53); fmul dword [0x804590]; fstp qword
}

// ===========================================================================================
// (4) 0x498a60 — four blocks × three packed-color channel normalizes byte·(1/255 f32), pushed f64.
// ===========================================================================================
//
// Each of the four blocks splits a color into three channel bytes (`movzx` from successive offsets),
// each `fild dword [ebp-0x4]; fmul dword [0x8026c8]=1/255(f32); fstp qword [esp+..]` — keeping the
// f64 product (no f32 narrow), straight into the `0x703f50` `"%f…"` formatter. 12 byte-identical
// `fmul dword [0x8026c8]` ops; one owned shape.

/// One 0x498a60 channel normalize (`fild dword [byte]; fmul dword [0x8026c8]; fstp qword`): the
/// channel byte (`0..=255`, zero-extended into a dword and `fild`'d — exact) `· (1/255 f32)` (widened
/// to f64), kept as the **f64** product (pushed as a `%f` double arg, NOT narrowed). `byte` is the
/// channel byte. Returns the f64 product. (Same shape as `framexml_units::color_channel`, with the
/// f32 `1/255` at `0x8026c8`.)
pub fn color_channel(byte: u8) -> f64 {
    f64::from(i32::from(byte)) * inv255_f32() // fild dword (exact); fmul dword [1/255 f32]; fstp qword
}
