//! Reimplements the arithmetic of the minimap frame functions `0x4eb0c0`,
//! `CGMinimapFrame::CGMinimapFrame` (`0x4edbc0`), `GetContainerItemCooldown` (`0x4f99b0`) and
//! `GetItemRepairCost` (`0x4faf30`). The x87 runs at 53-bit precision, so each chain is `f64`
//! with `as f32` at the `fstp dword` stores; the tests in `tests/minimap_frame.rs` diff each
//! against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants — read from WoW.exe (bit-exact; cited by VA; the difftest re-reads them from
// the mapped image where an op uses an absolute-addressed operand).
// ===========================================================================================

/// `[0x801608]` (`.rdata`) = `0.001` as an **f64** (`0x3f50624dd2f1a9fc`), 0x4f99b0's ms→seconds
/// cooldown normalize multiplier (`fmul qword [0x801608]`).
pub const MS_TO_SEC_F64_BITS: u64 = 0x3f50_624d_d2f1_a9fc;
/// `[0x7ff9d8]` (`.rdata`) = `1.0` as a **dword** `f32` (`0x3f800000`), 0x4faf30's `1.0 − alpha`
/// minuend (`fsubr dword [0x7ff9d8]`). Widened to f64.
pub const ONE_F32_BITS: u32 = 0x3f80_0000;
/// `[0x80655c]` (`.rdata`) = `5/3 ≈ 1.6666666` as a **dword** `f32` (`0x3fd55555`), 0x4edbc0's
/// unit-scale multiplier (`fmul dword [0x80655c]`). Widened to f64.
pub const K53_F32_BITS: u32 = 0x3fd5_5555;
/// `[0x807a4c]`,`[0x807a48]`,`[0x807a44]` (`.rdata`) = the three axis half-extent factors,
/// **dword** `f32`s `0.0125`/`0.00625`/`0.11` (`0x3c4ccccd`/`0x3bcccccd`/`0x3de147ae`). Widened.
pub const KX_F32_BITS: u32 = 0x3c4c_cccd;
pub const KY_F32_BITS: u32 = 0x3bcc_cccd;
pub const KZ_F32_BITS: u32 = 0x3de1_47ae;
/// `[0x7ffa24]` (`.rdata`) = `0.5` as a **dword** `f32` (`0x3f000000`), 0x4edbc0's halve multiplier
/// (`fmul dword [0x7ffa24]`). Widened. (Same `0.5` as the luabind quantize bias, used here as a
/// multiplier not an add.)
pub const HALF_F32_BITS: u32 = 0x3f00_0000;
/// `[0x807a3c]` (`.rdata`) = `0.005 ≈ 1/200` as a **dword** `f32` (`0x3ba3d70a`), 0x4eb0c0's
/// light-cull threshold factor (`fmul dword [0x807a3c]`). Widened.
pub const THRESH_K_F32_BITS: u32 = 0x3ba3_d70a;

#[inline]
fn ms_to_sec_f64() -> f64 {
    f64::from_bits(MS_TO_SEC_F64_BITS)
}
#[inline]
fn one_f32() -> f64 {
    f64::from(f32::from_bits(ONE_F32_BITS)) // f32 operand widened exactly to f64
}
#[inline]
fn k53_f32() -> f64 {
    f64::from(f32::from_bits(K53_F32_BITS))
}
#[inline]
fn kx_f32() -> f64 {
    f64::from(f32::from_bits(KX_F32_BITS))
}
#[inline]
fn ky_f32() -> f64 {
    f64::from(f32::from_bits(KY_F32_BITS))
}
#[inline]
fn kz_f32() -> f64 {
    f64::from(f32::from_bits(KZ_F32_BITS))
}
#[inline]
fn half_f32() -> f64 {
    f64::from(f32::from_bits(HALF_F32_BITS))
}
#[inline]
fn thresh_k_f32() -> f64 {
    f64::from(f32::from_bits(THRESH_K_F32_BITS))
}

// ===========================================================================================
// (1) 0x4faf30 — the per-item repair cost [.,0x4fafc0 ret]:
//     fsubr ×1 (0x4faf8f `1.0 − st0`), fmul ×1 (0x4fafac `base·mult`).
// ===========================================================================================

/// 0x4faf30's `1.0 − alpha` (`call 0x612b80 -> st0; fsubr dword [0x7ff9d8]=1.0; fstp dword [ebp-4]`).
/// The vendor price discount (`0x612b80`) leaves its f64 fraction on st0; `fsubr` SWAPS the operands
/// so the result is `1.0 − st0` (the f32 `1.0` minuend widened to f64), narrowed to f32 at the
/// `fstp dword`. `alpha` is taken as f32: exact for four of the five producible discounts, and for
/// `0.1f + 0.05f` (not an f32) both readings round to the same `0x3f59999a`. Returns the stored f32.
pub fn one_minus(alpha: f32) -> f32 {
    (one_f32() - f64::from(alpha)) as f32 // fsubr: [1.0] − st0  (PC_53 f64), then fstp dword
}

/// 0x4faf30's `count · frac` quantize (`fild qword [ebp-0xc]; fmul dword [ebp-0x4]; fstp dword
/// [ebp-0x4]; fld dword [ebp-0x4]; fistp dword [ebp-0x8]`). The DurabilityCosts base cost
/// (`0x5da330`) returns a u32 `count` in EAX, stored as a **qword** (hi-dword forced 0) and `fild`'d
/// (exact), multiplied by the f32 fraction `frac` (the `[ebp-0x4]` slot — either the default `1.0`
/// or the `one_minus` result), and the product narrowed to f32 at `fstp dword`. `count` is the base
/// cost; `frac` the f32 multiplier. Returns the f32
/// product (the value `fld`'d then `fistp`'d to the int return; the `fistp` convert is delegated to
/// `__ftol`-style rounding and is NOT arithmetic). NB the `fild qword` reads {lo=count, hi=0}.
pub fn count_scale(count: u32, frac: f32) -> f32 {
    (i64::from(count) as f64 * f64::from(frac)) as f32 // fild qword; fmul dword [frac]; fstp dword
}

// ===========================================================================================
// (2) 0x4f99b0 GetContainerItemCooldown [.,0x4f9b2a ret]:
//     fmul ×2 (0x4f9a95, 0x4f9ab4 — each `fild qword; fmul qword [0x801608]=0.001(f64); fstp qword`).
// ===========================================================================================

/// One 0x4f99b0 cooldown channel (`fild qword [ebp-8]; fmul qword [0x801608]; fstp qword [esp]`):
/// the marshalled ms value (a u32 stored as a **qword** with the high dword forced 0) is `fild`'d
/// (exact — non-negative, fits in the low dword) and multiplied by the **f64** `0.001` at `0x801608`;
/// the product keeps `f64` (pushed straight as the `lua_pushnumber` double arg). `ms` is the channel
/// dword. Returns the f64 product (seconds). (Two channels: cooldown start + duration; the third
/// pushed value is `fild`'d-only with no multiply — not an FP-arith op.)
pub fn cooldown_seconds(ms: u32) -> f64 {
    i64::from(ms) as f64 * ms_to_sec_f64() // fild qword (exact); fmul qword [0.001 f64]; fstp qword
}

// ===========================================================================================
// (3) 0x4edbc0 init glue [.,0x4edffa ret]:
//     fmul ×8, fchs ×2 — the unit-scale spread + halve + negate of three axis half-extents.
// ===========================================================================================

/// 0x4edbc0's `base = leaf(1.0) · 5/3` (`call 0x41ae70(0x3f800000) -> st0; fmul dword [0x80655c]`).
/// The unit-scale leaf result on st0 (`scale`) `· 5/3` (the f32 `0x80655c` widened to f64). Stays a
/// LIVE PC_53 f64 (`fVar6`) across the three axis multiplies. `scale` is the leaf result. Returns
/// the f64 `base`. (Computed twice in the body — same op — once for the half-extents, once at the
/// tail for `[param_1[0xce]+0x3a0]`; see [`tail_scale`].)
pub fn unit_base(scale: f32) -> f64 {
    f64::from(scale) * k53_f32() // fmul dword [0x80655c]=5/3  (PC_53 f64)
}

/// 0x4edbc0's three axis half-extents from the LIVE `base` (`fld dword [k]; fmul st,st(1)` for X,Y;
/// `fmul dword [k_z]` for Z — `base` consumed last). Each `k·base` (or `base·k`) narrows to f32 at its
/// `fstp dword`:
/// ```text
///   fld [0x807a4c]=k_x; fmul st,st(1)  -> (k_x·base)   ; fstp [0xbc7658]   = ext_x  (0x4edc53/.5d)
///   fld [0x807a48]=k_y; fmul st,st(1)  -> (k_y·base)   ; fstp [0xbc82a8]   = ext_y  (0x4edc7b/.88)
///   fmul [0x807a44]=k_z (consumes base)-> (base·k_z)   ; fstp [0xbc7970]   = ext_z  (0x4edca9)
/// ```
/// `base` is the LIVE f64 from [`unit_base`]. Returns `(ext_x, ext_y, ext_z)` as f32. (The
/// commutative-but-byte-distinct `fmul st,st(1)` vs `fmul dword [k_z]` both compute `k·base` at PC_53;
/// transcribed in source order.)
pub fn axis_extents(base: f64) -> (f32, f32, f32) {
    let ex = (kx_f32() * base) as f32; // fld [k_x]; fmul st,st(1); fstp dword
    let ey = (ky_f32() * base) as f32; // fld [k_y]; fmul st,st(1); fstp dword
    let ez = (base * kz_f32()) as f32; // fmul dword [k_z]; fstp dword (base consumed)
    (ex, ey, ez)
}

/// 0x4edbc0's halve of one axis half-extent (`fld dword [ext]; fmul dword [0x7ffa24]=0.5; fstp dword
/// [h]`): the f32 half-extent (reloaded, widened to f64) `· 0.5`, narrowed to f32 at the `fstp dword`.
/// Applied to all three (`0x4edcb5/.bb`, `0x4edcc7/.cd`, `0x4edcd9/.df`). `ext` is the half-extent.
/// Returns the f32 quarter-extent `h`.
pub fn half_extent(ext: f32) -> f32 {
    (f64::from(ext) * half_f32()) as f32 // fmul dword [0x7ffa24]=0.5; fstp dword
}

/// 0x4edbc0's negate of a halved extent (`fld dword [h]; fchs`): −0.0 hazard. The f32 `h`
/// reloaded/widened, `fchs` flips its sign bit (so `+0.0 -> -0.0`), kept LIVE f64 then duplicated/
/// stored as f32 across the matrix-fill stores. `h` is the quarter-extent. Returns the f32 `-h` (op-
/// for-op negation; `-(+0.0) == -0.0` matches `fchs`). Used for the X (`0x4edcf1`) and Y (`0x4edd7b`)
/// axes; the Z axis is stored without negation.
pub fn half_neg(h: f32) -> f32 {
    (-f64::from(h)) as f32 // fchs (sign flip, -0.0 on zero), kept f64; stored as f32
}

/// 0x4edbc0's tail scale (`call 0x41ae70(0x3f800000) -> st0; fmul dword [0x80655c]=5/3; fstp dword
/// [eax+0x3a0]`): identical op to [`unit_base`] but the product is narrowed to f32 at the `fstp
/// dword` (stored into the third object's `+0x3a0`). `scale` is the (re-fetched) leaf result.
/// Returns the f32 `scale·5/3`.
pub fn tail_scale(scale: f32) -> f32 {
    (f64::from(scale) * k53_f32()) as f32 // fmul dword [0x80655c]=5/3; fstp dword
}

// ===========================================================================================
// (4) 0x4eb0c0 render/pick driver [.,0x4eb87c ret 4]:
//     range loops (loop1 0x4eb1c0, loop2 0x4eb280): fadd/fsubr/fadd/fadd/fsub  per the disasm;
//     light loops (loop3 0x4eb399, loop4 0x4eb443): fmul[k]/fmul/fmul/fadd/fsubr/fabs ×2.
// ===========================================================================================
//
// The four owned regions all build a value from f32 memory operands (widened to f64) and feed it to
// an `fcomp` branch (the geometric range/visibility test). The owned FP-arith ops, transcribed as
// the binary computes them at PC_53.

/// 0x4eb0c0 range-loop coordinate sums (loop1 `fld [c]; fadd [off]`, loop2 same). `coord` and `off`
/// are f32 memory operands; the sum stays a LIVE PC_53 f64 (or, for the second axis, is narrowed to
/// f32 at a `fstp dword`). Returns the f64 sum. (`P = c3c + edx[-4]` stays live; `Q = c40 + edx[0]`
/// is `fstp`'d to f32 at `[ebp-0xc]`/`[ebp-0x44]` — see [`range_q`].)
pub fn coord_sum(coord: f32, off: f32) -> f64 {
    f64::from(coord) + f64::from(off) // fadd dword  (PC_53 f64)
}

/// 0x4eb0c0 range-loop second-axis sum stored to f32 (loop1 `fld [c40]; fadd [edx]; fstp [ebp-0xc]`,
/// loop2 `fld [c40]; fadd [c40off]; fstp [ebp-0x44]`). Same `fadd` as [`coord_sum`] but the result is
/// narrowed to f32 at the `fstp dword` before its later reloads. Returns the f32 `Q`.
pub fn range_q(coord: f32, off: f32) -> f32 {
    (f64::from(coord) + f64::from(off)) as f32 // fadd dword; fstp dword
}

/// 0x4eb0c0 range-loop low-edge test `st(1) − half` / `half + st(1)` and the second-axis
/// `half + q` / `q − half` (loop1 0x4eb1d4/.e6/.fc/.0c). `half` is the global half-cell
/// (`[0xbc762c]`, an f32-derived f64); `v` is the live sum `P` (or stored `Q`). The four owned ops:
/// * `fsubr st,st(1)` = `v − half`  (the SWAP: st(1) − st0, with st0=half) — [`sub_half`]
/// * `fadd st,st(1)`  = `half + v`                                          — [`add_half`]
/// * `fadd dword [Q]` = `half + Q`                                          — [`add_half`]
/// * `fsub dword [half]` = `Q − half`                                       — [`sub_half`]
///
/// All at PC_53 f64. `half`,`v` are the operands. Returns `v − half`.
pub fn sub_half(v: f64, half: f64) -> f64 {
    v - half // fsubr st,st(1) (= st1−st0) / fsub dword [half]  (PC_53 f64)
}
/// See [`sub_half`]. Returns `half + v` (the loop's `fadd st,st(1)` / `fadd dword`).
pub fn add_half(v: f64, half: f64) -> f64 {
    half + v // fadd  (PC_53 f64, commutative — order matches the binary's accumulate)
}

/// 0x4eb0c0 light-loop edge product (loop3 0x4eb3c0.., loop4 0x4eb443..):
/// ```text
///   fld [esi+0x7c]=scale ; fld [marker_x] ; fmul st,st(1) -> mx = marker_x·scale
///   fld [marker_y] ; fmul st,st(2) -> my = marker_y·scale ; fstp [ebp-0x4c]  (my, as f32)
///   fadd [c3c] -> (mx + c3c) ; fsubr [edi+0x24] -> (center_x − (mx+c3c)) ; fabs
///   fld [ebp-0x4c]=my ; fadd [c40] -> (my + c40) ; fsubr [edi+0x28] -> (center_y − (my+c40)) ; fabs
/// ```
/// The first product `mx` stays a LIVE PC_53 f64 through its `+c3c`, `fsubr`, `fabs`; the second
/// product `my` is narrowed to f32 (`fstp [ebp-0x4c]`) then reloaded before its `+c40`. So the X
/// edge uses the un-narrowed f64 `mx` while the Y edge uses the f32-narrowed `my` (live-vs-
/// reload asymmetry). `scale`=`[esi+0x7c]`, `mx_marker`/`my_marker` the per-light markers,
/// `cx`/`cy` the `[ebp-0x3c]`/`[ebp-0x40]` offsets, `center_x`/`center_y` the `[edi+0x24]`/`[edi+0x28]`
/// query point. Returns `(|center_x − (mx+cx)|, |center_y − (my_f32+cy)|)` as f64 (the two `fabs`
/// results compared against the threshold).
#[allow(clippy::too_many_arguments)]
pub fn light_edge(
    scale: f32,
    mx_marker: f32,
    my_marker: f32,
    cx: f32,
    cy: f32,
    center_x: f32,
    center_y: f32,
) -> (f64, f64) {
    let s = f64::from(scale);
    let mx = f64::from(mx_marker) * s; // fmul st,st(1); LIVE f64
    let my = (f64::from(my_marker) * s) as f32; // fmul st,st(2); fstp dword [ebp-0x4c] -> f32
    let ex = (f64::from(center_x) - (mx + f64::from(cx))).abs(); // fadd [c3c]; fsubr [edi+0x24]; fabs
    let ey = (f64::from(center_y) - (f64::from(my) + f64::from(cy))).abs(); // fld [ebp-0x4c]; fadd [c40]; fsubr [edi+0x28]; fabs
    (ex, ey)
}

/// 0x4eb0c0 light-cull threshold (`fld [esi+0x7c]=scale; fmul dword [0x807a3c]=0.005`): computed
/// ONCE on st1 before loop 3 and persisted across loops 3 AND 4 (loop 4 carries no own `fmul[k]`).
/// `scale` is `[esi+0x7c]`. Returns the f64 `scale·k` (the value the `fcomp st(1)` edge tests use).
pub fn scale_thresh(scale: f32) -> f64 {
    f64::from(scale) * thresh_k_f32() // fmul dword [0x807a3c]=0.005  (PC_53 f64)
}
