//! Reimplements the arithmetic of the map-point and taxi functions: the percent byte of
//! `SendChatMessage` (`0x49f1e0`), the point transform of `BuildMapLists` (`0x4a5d00`), and the
//! bound normalisation and route cost of `OpenTaxiMap` (`0x4db710`) and `RelaxTaxiRoute`
//! (`0x4dbce0`). The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the
//! `fstp dword` stores; the tests in `tests/worldmap_poi.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants — read from `WoW.exe` (bit-exact; cited by VA; the difftests assert each
// against the mapped image so a stale literal can't pass).
// ===========================================================================================

/// `[0x8029d0]` (`.rdata`) = `0.01` as a **dword** `f32` (`0x3c23d70a` = `0.009999999776482582`),
/// SendChatMessage's `byte · (1/100)` percent-normalize multiplier (`fmul dword [0x8029d0]`).
/// Widened to f64 in the multiply.
pub const INV100_F32_BITS: u32 = 0x3c23_d70a;

/// `0x4a5d00`'s five global f32 transform constants (all `.rdata` dwords, widened to f64).
/// `[0x806534]` = `32.0` (`0x42000000`), the scale pre-multiplier (`fmul dword [0x806534]` → `f2`).
pub const K806534_F32_BITS: u32 = 0x4200_0000;
/// `[0x806530]` = `31.3125` (`0x41fa8000`), the X bias origin (`fld; fsub st,st(1)` → `f1`).
pub const K806530_F32_BITS: u32 = 0x41fa_8000;
/// `[0x80652c]` = `20.875` (`0x41a70000`), the Y bias origin (`fld; fsub st,st(2)` → `l10`).
pub const K80652C_F32_BITS: u32 = 0x41a7_0000;
/// `[0x806528]` = `0.015968...` (`0x3c82cf75`), the X output scale (`fmul dword [0x806528]`).
pub const K806528_F32_BITS: u32 = 0x3c82_cf75;
/// `[0x806524]` = `0.023952...` (`0x3cc43730`), the Y output scale (`fmul dword [0x806524]`).
pub const K806524_F32_BITS: u32 = 0x3cc4_3730;

#[inline]
fn inv100_f32() -> f64 {
    f64::from(f32::from_bits(INV100_F32_BITS)) // f32 operand widened exactly to f64
}
#[inline]
fn k806534() -> f64 {
    f64::from(f32::from_bits(K806534_F32_BITS))
}
#[inline]
fn k806530() -> f64 {
    f64::from(f32::from_bits(K806530_F32_BITS))
}
#[inline]
fn k80652c() -> f64 {
    f64::from(f32::from_bits(K80652C_F32_BITS))
}
#[inline]
fn k806528() -> f64 {
    f64::from(f32::from_bits(K806528_F32_BITS))
}
#[inline]
fn k806524() -> f64 {
    f64::from(f32::from_bits(K806524_F32_BITS))
}

// ===========================================================================================
// (1) 0x49f1e0 SendChatMessage — one owned `fmul`: `fild dword [n]; fmul dword [0x8029d0]=0.01(f32);
//     fstp dword [esp]` (@ 0x49f63f..0x49f651). The byte at `[edx+0x1d]` is clamped to `<= 0x64`
//     (100), zero-extended into a dword, `fild`'d, `· (1/100 f32)`, narrowed to f32, and handed to
//     the `0x508680` `%`-format helper. NB the rest of the function (arg-marshal, net-send) is
//     delegated — this `fmul` is the sole owned FP-arith op in `[0x49f1e0, 0x49f794 ret)`.
// ===========================================================================================

/// `0x49f1e0`'s owned `fild dword [n]; fmul dword [0x8029d0]; fstp dword [esp]`: the clamped channel
/// byte (`0..=100`, zero-extended into a dword and `fild`'d — exact) `· (1/100 f32)`, narrowed to
/// f32 at the `fstp dword`. `byte` is the clamped `[edx+0x1d]` value. Returns the stored f32.
pub fn percent_byte(byte: u8) -> f32 {
    (f64::from(i32::from(byte)) * inv100_f32()) as f32 // fild dword; fmul dword [0.01]; fstp dword
}

// ===========================================================================================
// (2) 0x4a5d00 worldmap-POI transform — one big x87 block (0x4a5f42..0x4a5fe4), 17 owned FP-arith
//     ops. `eax` = POI record. Fields: `[eax+0x20]` = scale (f32); `[eax+0x18]`/`[eax+0x1c]` = f32
//     bias fields; `[eax+0x8]`/`[eax+0x10]` = int X/Y; `[eax+0xc]+1`/`[eax+0x14]+1` = int X1/Y1.
//     Stack discipline (traced from the disasm), with `s = [eax+0x20]`,
//     `k534=32.0`, `k530=31.3125`, `k52c=20.875`, `k528=0.015968`, `k524=0.023952`:
//
//     ```text
//       fld [eax+0x20]            -> st0=s
//       fmul [0x806534]          -> st0=f2 = s·k534                              (f2 LIVE, never spilled)
//       fld [0x806530]; fsub st,st(1); fadd [eax+0x18]
//                                -> st0=f1 = (k530−f2)+[eax+0x18], st1=f2        (f1 LIVE)
//       fld [0x80652c]; fsub st,st(2); fadd [eax+0x1c]
//                                -> st0=l10 = (k52c−f2)+[eax+0x1c], st1=f1, st2=f2
//       fstp [ebp-0xc]           -> SPILL l10 to f32, pop  -> st0=f1, st1=f2
//       fild [eax+0x8]; fmul [eax+0x20]; fadd st,st(1); fmul [0x806528]; fstp [out+0x10018]
//                                -> out0 = (i8·s + f1)·k528                      (f1 LIVE f64)
//       fild [eax+0xc]+1; fmul [eax+0x20]; fadd st,st(1); fmul [0x806528]; fstp [out+0x10020]
//                                -> out1 = ((ic+1)·s + f1)·k528
//       fstp st(0); fstp st(0)   -> drop f1, f2
//       fild [eax+0x10]; fmul [eax+0x20]; fadd [ebp-0xc]; fmul [0x806524]; fstp [out+0x10014]
//                                -> out2 = (i10·s + l10_f32)·k524               (l10 RELOADED f32)
//       fild [eax+0x14]+1; fmul [eax+0x20]; fadd [ebp-0xc]; fmul [0x806524]; fstp [out+0x1001c]
//                                -> out3 = ((iy+1)·s + l10_f32)·k524
//     ```
//
//     The `[ebx+ecx+...]`/`[ebx+edx+...]` store bases use `ecx=edx=[0xb6e668]` + the per-cell `ebx`
//     offset; the addresses are plumbing, the arithmetic is what's transcribed.
// ===========================================================================================

/// `0x4a5d00`'s owned POI coordinate transform. `scale` = `[eax+0x20]` (f32); `bias_x` = `[eax+0x18]`,
/// `bias_y` = `[eax+0x1c]` (f32); `ix0` = `[eax+0x8]`, `iy0` = `[eax+0x10]` (int); `ix1` = `[eax+0xc]`,
/// `iy1` = `[eax+0x14]` (int, each `+1`'d before use). Returns the four stored f32 edges in store
/// order `(out0 @+0x10018, out1 @+0x10020, out2 @+0x10014, out3 @+0x1001c)`.
///
/// `f1`/`f2` are kept LIVE as PC_53 f64 across the first two output stores; `l10` is spilled to f32
/// (`fstp dword [ebp-0xc]`) and reloaded for outputs 2/3 — modelled op-for-op (`l10_f32` round-trip).
pub fn poi_transform(
    scale: f32,
    bias_x: f32,
    bias_y: f32,
    ix0: i32,
    iy0: i32,
    ix1: i32,
    iy1: i32,
) -> (f32, f32, f32, f32) {
    let s = f64::from(scale);
    let f2 = s * k806534(); // fmul [0x806534]            (st0=f2, LIVE)
    let f1 = (k806530() - f2) + f64::from(bias_x); // fld [0x806530]; fsub st,st(1); fadd [eax+0x18]
    let l10 = (k80652c() - f2) + f64::from(bias_y); // fld [0x80652c]; fsub st,st(2); fadd [eax+0x1c]
    let l10_f32 = f64::from(l10 as f32); // fstp dword [ebp-0xc]  (SPILL to f32, reloaded for out2/3)

    // out0 = (i8·s + f1)·k528   (f1 added LIVE as f64)
    let out0 = ((f64::from(ix0) * s + f1) * k806528()) as f32;
    // out1 = ((ic+1)·s + f1)·k528
    let out1 = ((f64::from(ix1.wrapping_add(1)) * s + f1) * k806528()) as f32;
    // out2 = (i10·s + l10_f32)·k524   (l10 reloaded as f32)
    let out2 = ((f64::from(iy0) * s + l10_f32) * k806524()) as f32;
    // out3 = ((iy+1)·s + l10_f32)·k524
    let out3 = ((f64::from(iy1.wrapping_add(1)) * s + l10_f32) * k806524()) as f32;

    (out0, out1, out2, out3)
}

// ===========================================================================================
// (3) 0x4db710 region-bound glue — two kinds of owned FP-arith:
//
//  (A) two `(hi − lo)` bound subtracts feeding `!= 0.0` checks (0x4db839/0x4db86a):
//        fld [ebp-0x14]=hi_y; fsub [ebp-0x1c]=lo_y; fcomp [0x7ffd74]=0.0   ; (hi_y − lo_y) != 0
//        fld [ebp-0x18]=hi_x; fsub [ebp-0x20]=lo_x; fcomp [0x7ffd74]=0.0   ; (hi_x − lo_x) != 0
//      The `fcomp` is a compare (NOT FP-arith); the owned op is each `fsub`.
//
//  (B) two normalized ratios (0x4db958/0x4db97e), each `fld; fsub; fld; fsub; fdivp st(1),st`:
//        num_v = hi_x(0xb73a30) − coord_v([ecx+0xc]); den_v = hi_y(0xb73a34) − lo_y(0xb73a2c)
//          fdivp st(1),st -> num_v / den_v   (NATURAL order: st(1)=num built first / st(0)=den)
//        num_u = coord_u([ecx+8]) − lo_y(0xb73a2c); den_u = hi_x(0xb73a30) − lo_x(0xb73a28)
//          fdivp st(1),st -> num_u / den_u
//      (Decomp: `(fVar1 − [iVar4+0xc])/(a34 − a2c)` with `fVar1 = a30`, and
//               `([iVar4+8] − a2c)/(a30 − a28)`.) Each ratio = 2 `fsub` + 1 `fdivp`.
//
//  All operands are f32 memory, widened to f64; each ratio narrows to f32 at its `fstp dword`.
//  `true_ret = 0x4dbac0 (ret 4)`. Total owned FP-arith = 2 (A) + 6 (B) = 8.
// ===========================================================================================

/// One `0x4db710` bound subtract `(hi − lo)` (`fld dword [hi]; fsub dword [lo]`), the value the
/// source `fcomp`'s against `0.0`. Both operands are f32 memory widened to f64; the result stays
/// PC_53 f64 (consumed by the compare, never stored). `hi`/`lo` are the bound fields. Returns the
/// f64 difference (the exact value on `st0` at the `fcomp`).
pub fn bound_diff(hi: f32, lo: f32) -> f64 {
    f64::from(hi) - f64::from(lo) // fld dword [hi]; fsub dword [lo]  (PC_53 f64)
}

/// One `0x4db710` normalized ratio `num / den` where `num = na − nb`, `den = da − db`
/// (`fld [na]; fsub [nb]; fld [da]; fsub [db]; fdivp st(1),st; fstp dword [m32]`). The `fdivp
/// st(1),st` divides `st(1)` (the numerator, built first) by `st(0)` (the denominator) — NATURAL
/// order, NOT a swap. All four operands are f32 memory widened to f64; the quotient narrows to f32
/// at the `fstp dword`. Returns the stored f32 ratio.
///
/// For the first ratio (`→ [..+0xbb4f14]`): `na=hi_x`, `nb=coord_v`, `da=hi_y`, `db=lo_y`.
/// For the second (`→ [..+0xbb4f18]`): `na=coord_u`, `nb=lo_y`, `da=hi_x`, `db=lo_x`.
pub fn bounds_norm(na: f32, nb: f32, da: f32, db: f32) -> f32 {
    let num = f64::from(na) - f64::from(nb); // fld [na]; fsub [nb]
    let den = f64::from(da) - f64::from(db); // fld [da]; fsub [db]
    (num / den) as f32 // fdivp st(1),st  (num/den, natural order); fstp dword
}

// ===========================================================================================
// (4) 0x4dbce0 region-cost glue — one owned `fadd` (0x4dbe06): `<call 0x4dbbd0 = st0>; fadd dword
//     [edi+0xbb4f2c]; fstp dword [ebp-0x4]`. The delegated `0x4dbbd0` cost on `st0` is added to the
//     accumulated f32 cost field `[edi+0xbb4f2c]` (widened to f64), then narrowed to f32 and stored.
//     (Decomp: `fVar1 = (float)(FUN_004dbbd0() + (float10)(float)(&DAT_00bb4f2c)[..])`.) The `0x4dbbd0`
//     call is DELEGATED (separate row); this `fadd` is the sole owned FP-arith op in
//     `[0x4dbce0, 0x4dc002 ret)` (the later `fcomp [esi+0xbb4f2c]` is a compare, excluded).
// ===========================================================================================

/// `0x4dbce0`'s owned `fadd dword [edi+0xbb4f2c]; fstp dword [ebp-0x4]`: the `0x4dbbd0` call result
/// `c` (on `st0`, a PC_53 f64) `+` the accumulated f32 cost field `acc` (widened to f64), narrowed to
/// f32 at the `fstp dword`. `c` is the `st0` call result, `acc` the `[edi+0xbb4f2c]` f32. Returns the
/// stored f32 sum (matching the source's `fstp dword [ebp-0x4]`).
pub fn cost_add(c: f64, acc: f32) -> f32 {
    (c + f64::from(acc)) as f32 // fadd dword [edi+0xbb4f2c]; fstp dword [ebp-0x4]
}

/// `0x4dbce0`'s owned `fadd` as the un-narrowed PC_53 `f64` on `st0` immediately after the `fadd`
/// (before the `fstp dword` narrows it). Same op as [`cost_add`], read at the `fadd` site (the
/// faithful FP-arith observable, diffed by the copied-byte island). Returns the f64 sum.
pub fn cost_add_f64(c: f64, acc: f32) -> f64 {
    c + f64::from(acc) // fadd dword [edi+0xbb4f2c]  (PC_53 f64, pre-narrow)
}
