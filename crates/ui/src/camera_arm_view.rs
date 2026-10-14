//! Reimplements the arithmetic of `CCamera::SetView` (`0x512e90`) and of four functions in the
//! same band: `BakePortrait` (`0x524f60`), `CGTooltip::BuildUnit` (`0x529fe0`),
//! `CGTooltip::BuildSpell` (`0x52e610`) and `GameTooltip::SetInventoryItem` (`0x532ee0`). The x87
//! runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the
//! tests in `tests/camera_arm_view.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// Owned constants — read from `WoW.exe` (bit-exact; cited by VA). The difftest re-asserts each
// against the mapped image.
// ===========================================================================================

/// `[0x801360]` = `0.0010000000474974513` (`0x3a83126f`), the 0x512e90 near-zero guard epsilon
/// (`fcomp dword [0x801360]` — a compare, NOT arithmetic; documented for the guard model).
pub const EPS_512_BITS: u32 = 0x3a83_126f;
/// `[0x8029c8]` = `0.05000000074505806` (`0x3d4ccccd`) = the f32-rounded `1/20`, 0x529fe0's
/// percent-quantize multiplier (`fmul dword [0x8029c8]`). Widened to f64 in the multiply.
pub const PCT_MUL_BITS: u32 = 0x3d4c_cccd;
/// `[0x7ffa24]` = `0.5` (`0x3f000000`), 0x529fe0's round-to-nearest bias (`fadd dword [0x7ffa24]`).
pub const HALF_BITS: u32 = 0x3f00_0000;
/// `[0x808ff0]` = `60000.0` (`0x476a6000`), 0x52e610's region-1 divisor when `esi != 0`.
pub const C60000_BITS: u32 = 0x476a_6000;
/// `[0x8022cc]` = `1000.0` (`0x447a0000`), 0x52e610's region-1 divisor when `esi == 0`.
pub const C1000_BITS: u32 = 0x447a_0000;
/// `[0x8015b8]` = `1.0` as a **qword** `f64` (`0x3ff0000000000000`), SetInventoryItem's slot
/// 1→0 base offset (`fsub qword [0x8015b8]`). The full-width f64 one.
pub const ONE_F64_BITS: u64 = 0x3ff0_0000_0000_0000;

// ===========================================================================================
// (1) 0x512e90 — camera-pitch re-centre (fadd ×1) + four near-zero ABS guards (fsub/fabs).
// ===========================================================================================
//
// In the `param_3 == 1` branch, after a virtual getter / `fld [ebx+0xc98]` leaves a current pitch
// on st0:
//   fadd [ebp+8]=param2; push ecx; fstp dword [esp]; call 0x455b30 (clamp/normalize, DELEGATED)
//     → [ebp+8] = (f32)(pitch + param2)   — the owned re-centre.
// In the `param_3 == 2` branch, four `_DAT_00801360 <= ABS(..)` near-zero guards each form
//   fld [esi+off]; fsub [coord]; fabs; fcomp [0x801360]; fnstsw ax; test ah,5; jp
// (guard D omits the fsub — it is `fabs([esi+0x104])` alone). The `fcomp` and the `test ah,5; jp`
// implement `eps <= |x|` ⇒ the guard's boolean. The owned arith is the fsub (f32−f32 widened) and
// the fabs; the compare disposition is modelled in [`guard_flags`].

/// 0x512e90's pitch re-centre (`fadd dword [ebp+8]; fstp dword [esp]`): the current pitch `pitch`
/// (a PC_53 f64 left on st0 by the getter) plus `param2` (the f32 target, widened), narrowed to
/// f32 at the `fstp dword`. `pitch` is the live st0 value; `param2` the f32 arg. Returns the f32.
pub fn pitch_recenter(pitch: f64, param2: f32) -> f32 {
    (pitch + f64::from(param2)) as f32
}

/// 0x512e90's near-zero guard arithmetic (`fld [esi+off]; fsub [coord]; fabs`): `|field − coord|`
/// where both operands are f32 widened to f64. `field`/`coord` are the two f32 inputs. Returns the
/// f64 magnitude (the value the source `fabs` produces on st0 before the `fcomp`).
pub fn guard_abs_sub(field: f32, coord: f32) -> f64 {
    (f64::from(field) - f64::from(coord)).abs() // fld; fsub dword; fabs  (PC_53 f64)
}

/// 0x512e90 guard D's lone `fabs` (`fld [esi+0x104]; fabs`): `|field|` (f32 widened). Returns the
/// f64 magnitude.
pub fn guard_abs(field: f32) -> f64 {
    f64::from(field).abs()
}

/// The x87 `fnstsw` AX value a `... ; fabs; fcomp dword [0x801360]` leaves for 0x512e90's guards,
/// packed as the binary reads it (`test ah,5; jp`). For the magnitude `m` vs eps `e`: C0 (SW bit 8,
/// AH bit 0) = `m < e`; C2 (SW bit 10, AH bit 2) = unordered (never, for finite inputs); C3 (SW
/// bit 14) = `m == e`. The guard's branch is `test ah,5 (C0|C2); jp` (parity of {C0,C2}); for
/// finite inputs C2=0, so it is the parity of C0 alone = C0. Returns the 16-bit AX value (the bits
/// the binary's `fnstsw ax` writes; the difftest compares the masked `ah & 5`).
pub fn guard_flags(m: f64, eps_bits: u32) -> u16 {
    let e = k(eps_bits);
    let c0 = u16::from(m < e) << 8; // SW bit 8
    let c3 = u16::from(m == e) << 14; // SW bit 14
    c0 | c3
}

// ===========================================================================================
// (2) 0x524f60 — two scale-ratio reconstructions over the 0x58a250-filled f32 buffer.
// ===========================================================================================
//
// Region 1 (`0x5251b7`..`0x525224`), the leaf helpers leaving values LIVE on the x87 stack:
//   fld [ebp-0x38]; fsub [ebp-0x40]; fstp [ebp-0x4]      ; d0 = (f32)([ebp-0x38] − [ebp-0x40])
//   fld [ebp-0x3c]; fsub [ebp-0x44]; fstp [ebp-0x28]     ; d1 = (f32)([ebp-0x3c] − [ebp-0x44])
//   call 0x41ae60 → st0 = m0 = 0.8·arg            (leaf, DELEGATED; result LIVE)
//   fld [0x808de4]=64; fdiv [ebp-0x4]=d0; fmulp st(1),st ; st0 = m0 · (64 / d0)  → [ebp-0xc]
//   call 0x41ae70 → st0 = m1 = 0.6·arg            (leaf; result stored raw → [ebp-0x34])
//   call 0x41ae70 → st0 = m2                       (→ [ebp-0x4], reusing the d0 slot)
//   call 0x41ae70 → st0 = m3; fdiv [ebp-0x28]=d1; fmul [0x808de4]=64; fsubr [ebp-0x4]=m2
//     → [ebp-0x24] = (f32)( m2 − ((m3 / d1) · 64) )
// Region 2 (`0x525414`..`0x525430`):
//   fld [ebp-0x18]; fsub [ebp-0x20]; fld [ebp-0x1c]; fsub [ebp-0x24]; fdivp st(1),st
//     → [ebp-0xc] = (f32)( ([ebp-0x18] − [ebp-0x20]) / ([ebp-0x1c] − [ebp-0x24]) )
// The leaves' inputs (the `push 0x3f800000` = 1.0 args) and orchestration are DELEGATED; the owned
// slices below model exactly the fsub/fdiv/fmulp/fmul/fsubr/fdivp the function performs. The difftest
// runs the leaf bytes natively inside its slices, so the Rust models receive the leaf results
// (`m0..m3`) as inputs.

/// 0x524f60's owned f32 delta (`fld [a]; fsub [b]; fstp [m32]`): `(a − b) as f32`. Used by the two
/// region-1 deltas (`d0`, `d1`) and is the same shape as region-2's two fld/fsub inputs.
pub fn buf_delta(a: f32, b: f32) -> f32 {
    (f64::from(a) - f64::from(b)) as f32
}

/// 0x524f60 region-1 product#1 (`fld [0x808de4]=64; fdiv [ebp-0x4]=d0; fmulp st(1),st; fstp [m32]`):
/// `(m0 · (64 / d0)) as f32`, where `m0` is the LIVE leaf-`0x41ae60` result on st1 and `d0` the f32
/// delta. The `fld` pushes 64 above `m0`; `fdiv [d0]` → `64/d0`; `fmulp st(1),st` → `m0·(64/d0)`,
/// pop. `c64_bits` is `[0x808de4]` (= 64.0). Returns the f32 stored.
pub fn ratio_product1(m0: f64, d0: f32, c64_bits: u32) -> f32 {
    let q = k(c64_bits) / f64::from(d0); // fdiv dword [ebp-0x4]
    (m0 * q) as f32 // fmulp st(1),st  (m0 LIVE f64 on st1)
}

/// 0x524f60 region-1 product#2 (`fdiv [ebp-0x28]=d1; fmul [0x808de4]=64; fsubr [ebp-0x4]=m2;
/// fstp [m32]`): `(m2 − ((m3 / d1) · 64)) as f32`. `m3` is the LIVE leaf-`0x41ae70` result on st0,
/// `d1` the f32 delta, `m2` the f32 reloaded operand (the prior leaf result stored to `[ebp-0x4]`).
/// `fdiv` → `m3/d1`; `fmul [64]` → `·64`; `fsubr [m2]` → `m2 − that` (reverse-subtract: the memory
/// operand is the minuend). `c64_bits` is `[0x808de4]` (= 64.0). Returns the f32 stored.
pub fn ratio_product2(m3: f64, d1: f32, m2: f32, c64_bits: u32) -> f32 {
    let t = (m3 / f64::from(d1)) * k(c64_bits); // fdiv dword [ebp-0x28]; fmul dword [0x808de4]
    (f64::from(m2) - t) as f32 // fsubr dword [ebp-0x4]  (m2 − t)
}

/// 0x524f60 region-2 ratio (`fld [a]; fsub [b]; fld [c]; fsub [d]; fdivp st(1),st; fstp [m32]`):
/// `((a − b) / (c − d)) as f32`. `fdivp st(1),st` divides st1 (= a−b) by st0 (= c−d), pop — so the
/// FIRST delta is the dividend. Returns the f32 stored.
pub fn region2_ratio(a: f32, b: f32, c: f32, d: f32) -> f32 {
    let num = f64::from(a) - f64::from(b); // fld [a]; fsub [b]   (st1)
    let den = f64::from(c) - f64::from(d); // fld [c]; fsub [d]   (st0)
    (num / den) as f32 // fdivp st(1),st  → st1 / st0
}

// ===========================================================================================
// (3) 0x529fe0 — percent-quantize  (i32)(n·0.05 + 0.5), n clamped 0..=100.
// ===========================================================================================
//
//   mov al,[edx+0x1d]; cmp al,0x64; jb +2; mov al,0x64  ; n = min(byte, 100)
//   movzx eax,al; mov [ebp-0x18],eax; fild [ebp-0x18]   ; st0 = (i32)n
//   fmul dword [0x8029c8]=0.05; fadd dword [0x7ffa24]=0.5; call __ftol  ; (i32)(n·0.05 + 0.5)
// The byte clamp is integer (orchestration); the owned arith is `fmul` + `fadd`.

/// 0x529fe0's percent-quantize (`fild [n]; fmul dword [0x8029c8]; fadd dword [0x7ffa24]; __ftol`):
/// `(i32)(n·0.05 + 0.5)` truncate-toward-zero, where `n` is the clamped byte (0..=100, fild'd
/// exact). `mul_bits` = `[0x8029c8]` (= 0.05 f32), `half_bits` = `[0x7ffa24]` (= 0.5 f32), both
/// widened. Returns the truncated i32 (the `__ftol` result in EAX).
pub fn percent_quantize(n: u8, mul_bits: u32, half_bits: u32) -> i32 {
    let p = f64::from(i32::from(n)) * k(mul_bits) + k(half_bits); // fmul; fadd  (PC_53 f64)
    benilla_re_cmath::x87::ftol(p) // __ftol truncate toward zero
}

// ===========================================================================================
// (4) 0x52e610 — two timer ratios n / C, stored as f64 (qword).
// ===========================================================================================
//
//   (esi ? fld [0x808ff0]=60000 : fld [0x8022cc]=1000)   ; st0 = C
//   fild [ebp-d]; sub esp,8; fdiv st,st(1); fstp qword [esp]; fstp st(0)
//     → [esp] = (f64)( n / C )   (n the fild'd int on st0, C the LIVE constant on st1)
// The result is pushed as the `0x64a7f0` formatter's double arg — kept f64 (no narrowing).

/// 0x52e610's timer ratio (`fld [C]; fild [n]; fdiv st,st(1); fstp qword`): `n / C` as a **f64**
/// (no narrowing — `fstp qword`). `fdiv st,st(1)` divides st0 (= the fild'd int `n`) by st1 (= the
/// constant `C`). `n` is the int, `c_bits` the f32 constant (60000 or 1000) widened to f64.
/// Returns the f64 quotient.
pub fn timer_ratio(n: i32, c_bits: u32) -> f64 {
    f64::from(n) / k(c_bits) // fdiv st,st(1)  → st0(n) / st1(C)
}

// ===========================================================================================
// (5) 0x532ee0 SetInventoryItem — slot index = (i32)(lua_arg − 1.0).
// ===========================================================================================
//
//   mov edx,3; mov ecx,esi; call 0x6f3620 (lua_tonumber → double on st0, DELEGATED)
//   fsub qword [0x8015b8]=1.0; call 0x40a2b0 (__ftol)   ; slot = (i32)(arg − 1.0)
// The Lua marshalling + the bag/slot dispatch are DELEGATED; the owned arith is the single `fsub`.

/// 0x532ee0's slot conversion (`fsub qword [0x8015b8]; __ftol`): `(i32)(arg − 1.0)`
/// truncate-toward-zero, where `arg` is the `lua_tonumber` f64 result (the 1-based Lua inventory
/// slot) and the subtrahend is the full-width f64 `1.0` at `0x8015b8`. Returns the truncated i32
/// (the `__ftol` result in EAX), the 0-based slot index.
pub fn inv_slot(arg: f64) -> i32 {
    benilla_re_cmath::x87::ftol(arg - f64::from_bits(ONE_F64_BITS)) // fsub qword; __ftol
}
