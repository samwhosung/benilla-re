//! Reimplements the arithmetic of the simple-model functions `SimpleModelSetModel` (`0x76cd30`),
//! the geometry getters (`0x76d080`, `0x76d0d0`), `SimpleModelSetAlpha` (`0x76d120`),
//! `SimpleModelUpdateTransform` (`0x76d1a0`), `SimpleModelDraw` (`0x76d240`) and `0x76d7f0`. The
//! x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores;
//! the tests in `tests/simplemodel.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

/// 1/255 = 0.0039215689 (`DAT_008026c8`), the byte→[0,1] colour normalize.
pub const INV255_BITS: u32 = 0x3b80_8081;
/// 255.0 (`DAT_007ffe58`), the [0,1]→byte colour scale.
pub const C255_BITS: u32 = 0x437f_0000;
/// 0.5 (`DAT_007ffa24`), the colour-pack round bias.
pub const HALF_BITS: u32 = 0x3f00_0000;
/// 1000.0 (`DAT_008022cc`), the `__ftol` unit scale.
pub const SCALE1000_BITS: u32 = 0x447a_0000;
/// 5/3 = 1.6666666 (`DAT_0080655c`), the 0x76d1a0 base scale.
pub const FIVE_THIRDS_BITS: u32 = 0x3fd5_5555;
/// 0.6 (`DAT_00832a48`), the leaf-helper `0x41ae70` const (`0.6·arg`).
pub const POINT6_BITS: u32 = 0x3f19_999a;
/// norm-ε = 2⁻²² (`DAT_008029d4`), the change-detect epsilon.
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07

#[inline]
fn inv255() -> f64 {
    f64::from(f32::from_bits(INV255_BITS))
}

// ===========================================================================================
// (1) byte→[0,1] colour normalize — 0x76cd30 / 0x76d120 (`fild [byte]; fmul [1/255]; fstp`)
// ===========================================================================================

/// The byte→unit normalize of `0x76cd30` (`0x76cd6b`..`0x76cd77`) and its byte-identical clone
/// `0x76d120` (`0x76d144`..`0x76d150`): a `byte` field (`movzx [esi+0xc8]`) is `fild`'d to f64 and
/// scaled by `1/255`, the product stored `fstp dword [esp]` → **f32** and pushed to the setter
/// `0x710cb0`. `b` is the 0..=255 alpha byte.
pub fn byte_to_unit(b: u8) -> f32 {
    (f64::from(i32::from(b)) * inv255()) as f32
}

// ===========================================================================================
// (2) flag-gated field difference — 0x76d080 / 0x76d0d0 (`fld [a]; fsub [b]`)
// ===========================================================================================

/// The positive-branch field difference of `0x76d080` (`0x76d0b5`..`0x76d0bb`, returns
/// `[+0x39c] − [+0x390]`) and its clone `0x76d0d0` (`0x76d105`..`0x76d10b`, `[+0x3a0] − [+0x394]`).
/// Both reach this `fld [a]; fsub [b]` only when a delegated getter returned a value `> 0.0` AND a
/// sub-flag check passed; the returned difference is left LIVE in st0 (NO store — the caller
/// consumes the x87 value), so it is the un-narrowed PC_53 f64. `a`/`b` are the two `f32` fields.
pub fn field_diff(a: f32, b: f32) -> f64 {
    f64::from(a) - f64::from(b)
}

// ===========================================================================================
// (3) 0x76d1a0 — base-scale × field product + per-component scale (6 fmul)
// ===========================================================================================

/// `0x76d1a0` base scale (`0x76d1c1`..`0x76d1e7`): `base = ((0.6·1.0)·(5/3)) · ([+0x3a0]·[+0x7c])`.
/// The leaf `0x41ae70(1.0)` returns `0.6·1.0`; `fmul [5/3]` (op1), `fld [+0x3a0]; fmul [+0x7c]`
/// (op2), `fmulp st(1),st` (op3) folds the two terms. Stored `fst [ebp-0xc]`/`[ebp-8]`/`fstp
/// [ebp-4]` → **f32** (three stores of the same value). `f3a0`/`f7c` are the two `f32` fields.
pub fn d1a0_base(f3a0: f32, f7c: f32) -> f32 {
    let r = f64::from(f32::from_bits(POINT6_BITS)) * 1.0; // leaf 0x41ae70(1.0) = 0.6·1.0
    let r = r * f64::from(f32::from_bits(FIVE_THIRDS_BITS)); // op1: ·5/3
    let term = f64::from(f3a0) * f64::from(f7c); // op2: [+0x3a0]·[+0x7c]
    (r * term) as f32 // op3: fmulp; fst → f32
}

/// `0x76d1a0` per-component scale (`0x76d205`..`0x76d232`): the field `[+0x7c]` scaled by each of
/// `[+0x390]/[+0x394]/[+0x398]`, each product stored `fstp dword` → **f32** (`[ebp-0x24]/-0x20/
/// -0x1c`). Returns `[f7c·c390, f7c·c394, f7c·c398]`. (op4/op5/op6 — the `fld [+0x7c]` is re-dup'd
/// per component via `fld st(0)`, so all three multiply the SAME f7c.)
pub fn d1a0_components(f7c: f32, c390: f32, c394: f32, c398: f32) -> [f32; 3] {
    let v = f64::from(f7c);
    [
        (v * f64::from(c390)) as f32,
        (v * f64::from(c394)) as f32,
        (v * f64::from(c398)) as f32,
    ]
}

// ===========================================================================================
// (4) 0x76d240 — clamp-pair eps change-detect (2 fsub+2 fabs) + 4-fmul scale (slices A,B)
// ===========================================================================================

/// `0x76d240` slice-A change-detect (`0x76d574`..`0x76d58d` / `0x76d58d`..`0x76d5a6`): after each
/// of the four candidate slots is clamped to {0.0, 1.0}, two `fld [a]; fsub [b]; fabs; fcomp
/// [eps=2⁻²²]; fnstsw; test ah,5; jnp 0x76d63e` change-detect tests fire. `test ah,5` masks {C0,C2}
/// — **PF=1 ⟺ EVEN set-bits** — so `jnp` (PF=0) is taken iff ODD set-bits, i.e. `|a−b| < eps`
/// ordered (C0=1, 1 bit) → jumps to the no-op tail; falls through (does the work) iff `|a−b| >= eps`
/// / `== eps` (C3, 0 masked bits, even) / NaN (C0|C2, 2 bits, even). Returns `true` ⇒ `|a−b| < eps`
/// (false on NaN) ⇒ this pair is settled (the binary SKIPS the body). `a`/`b` are the clamped
/// pair values.
pub fn d240_settled(a: f32, b: f32) -> bool {
    let d = (f64::from(a) - f64::from(b)).abs();
    !d.is_nan() && d < f64::from(f32::from_bits(EPS_BITS))
}

/// `0x76d240` slice-B scaled products (`0x76d5a6`..`0x76d5d7`): four `fld [v]; fmul [s]; fstp
/// dword` products pushed as the four args to `0x58af60` (in stack order
/// `[esp+0xc],[esp+8],[esp+4],[esp]`):
/// `[f8·s18, f10·s18, f4·s14, fc·s14]` where `s18=[ebp-0x18]`, `s14=[ebp-0x14]` are the matrix-kernel
/// outputs and `f4/f8/fc/f10` the clamped slots. Each `fstp dword` → **f32**. Returns the four f32
/// in push order `[f8·s18, f10·s18, f4·s14, fc·s14]`.
pub fn d240_scaled(f8: f32, f10: f32, f4: f32, fc: f32, s18: f32, s14: f32) -> [f32; 4] {
    [
        (f64::from(f8) * f64::from(s18)) as f32,
        (f64::from(f10) * f64::from(s18)) as f32,
        (f64::from(f4) * f64::from(s14)) as f32,
        (f64::from(fc) * f64::from(s14)) as f32,
    ]
}

// ===========================================================================================
// (5) 0x76d680 — colour pack: per-channel clamp[0,1] · 255 + 0.5 → __ftol → 0xBBGGRR (6 fmul/fadd)
// ===========================================================================================

/// One colour channel of `0x76d680`'s pack (e.g. `0x76d6ac`..`0x76d6eb`): the unpacked-channel f32
/// is clamped to `[0,1)` then scaled, with `__ftol` truncating toward zero → a byte. The two clamp
/// gates, reconstructed from the EMITTED jcc (NOT the mask):
/// * `0x76d6af fcomp 0.0; test ah,0x41(={C0,C3}); jp 0x76d6c4` — `jp` (PF=1) taken iff EVEN
///   set-bits. `c < 0` → C0=1 (1 bit, odd) → NOT taken → loads `0.0` (clamp low). `c >= 0`
///   (C0=0,C3=0, even) → taken; `c == 0` (C3=1, odd) → NOT taken → `0.0`; NaN (C0|C2|C3, 2 masked
///   bits, even) → taken.
/// * `0x76d6c7 fcomp 1.0; test ah,1(={C0}); jne 0x76d6dc` — `jne` (ZF: masked AH != 0) taken iff
///   `c < 1` (C0=1). `c >= 1` (C0=0) → NOT taken → loads `255.0` (clamp high). NaN sets C0=1 →
///   taken → the in-range `c·255 + 0.5` path with `c = NaN`.
///
/// In-range path `0x76d6dc`: `fld c; fmul [255]; fadd [0.5]`. So:
/// * `c < 0` (or `== 0`) → `__ftol(0.0) = 0`,
/// * `c >= 1` → `__ftol(255.0) = 255`,
/// * else → `__ftol(c·255 + 0.5)` (truncate),
/// * NaN → `__ftol(NaN)` = the x87 integer-indefinite `0x8000_0000` (low byte `0x00`).
///
/// Returns the byte the corresponding `__ftol` produces. (NaN's `__ftol` is `0x80000000`; its low
/// byte is `0` — diffed explicitly.)
pub fn d680_channel(c: f32) -> u8 {
    let v = f64::from(c);
    if v.is_nan() {
        // in-range path with c=NaN ⇒ NaN·255+0.5 = NaN ⇒ __ftol = 0x80000000, low byte 0x00.
        return 0;
    }
    let scaled = if v <= 0.0 {
        0.0 // c < 0 OR c == 0 (first gate's jp-not-taken) ⇒ clamp low
    } else if v < 1.0 {
        v * f64::from(f32::from_bits(C255_BITS)) + f64::from(f32::from_bits(HALF_BITS))
    // in range
    } else {
        255.0 // c >= 1.0 ⇒ clamp high (the loaded 255.0 const)
    };
    // __ftol truncates toward zero; all non-NaN results are in [0,255], non-negative.
    scaled.trunc() as i64 as u8
}

/// `0x76d680` full colour pack (`0x76d6ac`..`0x76d79a`): the three unpacked channels at
/// `[ebp-8]`/`[ebp-0xc]`/`[ebp-0x10]` are clamp-and-scaled in that order, each result LEFT ON the
/// x87 stack (no pop between blocks). At `0x76d769`–`0x76d791` three `__ftol` calls POP the stack in
/// LIFO order — st0 first = the LAST-computed = `[ebp-0x10]`, then `[ebp-0xc]`, then `[ebp-8]` —
/// each result OR'd into EDI MSB→LSB:
/// `edi = ((0xffffff00 | ftol(a10)) << 8 | ftol(ac)) << 8 | ftol(a8)`.
/// So the low 24 bits carry `(byte(a10)<<16)|(byte(ac)<<8)|byte(a8)` and the high bits the
/// sign-extended `0xffffff00` carry — returned as the full u32 EDI. `a8`/`ac`/`a10` are the three
/// `f32` channel slots (`[ebp-8]`/`[ebp-0xc]`/`[ebp-0x10]`).
pub fn d680_pack(a8: f32, ac: f32, a10: f32) -> u32 {
    let b8 = u32::from(d680_channel(a8));
    let bc = u32::from(d680_channel(ac));
    let b10 = u32::from(d680_channel(a10));
    let mut edi: u32 = 0xffff_ff00 | b10; // 1st __ftol pops st0 = the last-computed slot [ebp-0x10]
    edi <<= 8;
    edi |= bc; // 2nd __ftol = [ebp-0xc]
    edi <<= 8;
    edi |= b8; // 3rd __ftol = [ebp-8]
    edi
}

// ===========================================================================================
// (6) 0x76d7f0 — unit-convert via __ftol (`fmul [1000]; __ftol`)
// ===========================================================================================

/// `0x76d7f0` unit-convert (`0x76d843`..`0x76d854`): `fld [ebp+8]=arg; fmul [0x8022cc]=1000.0;
/// fcom 0.0; fnstsw; __ftol`. The product `arg·1000.0` is `__ftol`'d — **truncate toward zero**.
/// (The `fcom 0.0` only sets flags for a sign check the caller never reads here; the `fnstsw` AH is
/// discarded.) `arg` = the `f32` pushed by the caller.
pub fn d7f0_unit_convert(arg: f32) -> i32 {
    let v = f64::from(arg) * f64::from(f32::from_bits(SCALE1000_BITS));
    v.trunc() as i32
}
