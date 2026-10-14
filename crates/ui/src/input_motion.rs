//! Reimplements the arithmetic of `0x503c10`, `0x5132f0`, `CInputControl::AccumulateMotion`
//! (`0x514400`) and `CInputControl::HandOffPitch` (`0x515330`). The x87 runs at 53-bit precision,
//! so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in
//! `tests/input_motion.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_00808300` = 0.33 (`0x3ea8f5c3`), the 0x503c10 chain-1 `·0.33` scale.
pub const K033_BITS: u32 = 0x3ea8_f5c3;
/// `DAT_007ffa24` = 0.5 (`0x3f000000`), the 0x503c10 chain-2 `·0.5` scale.
pub const K05_BITS: u32 = 0x3f00_0000;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x503c10 — two independent `(a − b)·k → f32` offset-scale stores.
// ===========================================================================================

/// The single `fld [a]; fsub [b]; fmul [k]; fstp [m32]` offset-scale shape shared by both of
/// `0x503c10`'s owned products. Both inputs `a`,`b` are `f32` locals (filled by the prior
/// `0x711a20` getter); the subtract chains `f64` on st0 through the `fmul`, rounding to `f32`
/// only at the `fstp`. `a`,`b` are the two operands, `kbits` the scale constant. Returns the f32.
pub fn offset_scale(a: f32, b: f32, kbits: u32) -> f32 {
    ((f64::from(a) - f64::from(b)) * k(kbits)) as f32
}

/// The full two-store sequence of `0x503c10`, in disasm order:
/// * `out0 = (m38 − m44)·0.33` → `[ebp-0x14]` (`fld [ebp-0x38]; fsub [ebp-0x44]; fmul [0.33]`)
/// * `out1 = (m34 − m40)·0.5`  → `[ebp-0x10]` (`fld [ebp-0x34]; fsub [ebp-0x40]; fmul [0.5]`)
///
/// The two chains are independent (no register carry — each is a fresh `fld`). `a0`/`b0` are
/// `[ebp-0x38]`/`[ebp-0x44]`; `a1`/`b1` are `[ebp-0x34]`/`[ebp-0x40]`. Returns `(out0, out1)`.
pub fn offset_scale2(a0: f32, b0: f32, a1: f32, b1: f32) -> (f32, f32) {
    (
        offset_scale(a0, b0, K033_BITS),
        offset_scale(a1, b1, K05_BITS),
    )
}

// ===========================================================================================
// (2) 0x5132f0 — in-place scale of a 3-element f32 vector by a scalar arg.
// ===========================================================================================

/// `0x5132f0`'s owned arithmetic (`0x5132f3`..`0x51330e`): the `this` (ecx) points at a 3-element
/// f32 vector `[v0,v1,v2]` at offsets `+0`/`+4`/`+8`; each element is replaced by `arg · v_i`
/// (`fld [ebp+8]; fmul [eax+N]; fstp [eax+N]`, N=0/4/8). Each product is an independent chain
/// (the scalar is re-`fld`'d each time), `f64` on st0 until its `fstp` rounds to `f32`. `arg` is
/// `[ebp+8]`, `v` the input vector. Returns the three scaled f32 elements `(arg·v0, ·v1, ·v2)`.
pub fn vec3_scale(arg: f32, v: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(arg) * f64::from(v[0])) as f32,
        (f64::from(arg) * f64::from(v[1])) as f32,
        (f64::from(arg) * f64::from(v[2])) as f32,
    ]
}

// ===========================================================================================
// (3) 0x514400 — accumulate `|src.x|`, `|src.y|` into a running `(x,y)` field pair.
// ===========================================================================================

/// `0x514400`'s owned arithmetic (`0x51441c`..`0x514435`): from a source struct (`edi = [ebp+8]`)
/// it reads `src.x = [edi+0x24]`, `src.y = [edi+0x28]`, takes their absolute value, and ADDS each
/// into the running accumulator fields of `this` (`esi`): `esi.x = [esi+0x8]`, `esi.y = [esi+0xc]`:
/// * `acc_x' = |src.x| + acc_x` → `[esi+0x8]`  (`fld [edi+0x24]; fabs; fadd [esi+0x8]; fstp`)
/// * `acc_y' = |src.y| + acc_y` → `[esi+0xc]`  (`fld [edi+0x28]; fabs; fadd [esi+0xc]; fstp`)
///
/// `fabs` is exact; the add chains `f64` on st0 (acc widened from its stored `f32`) until the
/// `fstp` rounds to `f32`. `sx`/`sy` are `src.x`/`src.y`, `ax`/`ay` the prior accumulators.
/// Returns `(acc_x', acc_y')`.
pub fn abs_accumulate(sx: f32, sy: f32, ax: f32, ay: f32) -> (f32, f32) {
    let acc_x = (f64::from(sx).abs() + f64::from(ax)) as f32;
    let acc_y = (f64::from(sy).abs() + f64::from(ay)) as f32;
    (acc_x, acc_y)
}

// ===========================================================================================
// (4) 0x515330 — negate one arg `f32` and emit it as a call argument.
// ===========================================================================================

/// `0x515330`'s owned arithmetic (`0x515364`..`0x51536f`): `fld [ebp+0xc]; fchs; fstp [esp]` —
/// the second arg (`[ebp+0xc]`) is negated and pushed as the (first) call argument to `0x60de70`.
/// `fchs` is the exact sign-bit flip of the `f32`; the `fstp` stores it back as `f32`. `arg` is
/// `[ebp+0xc]`. Returns `−arg`.
pub fn negate(arg: f32) -> f32 {
    (-f64::from(arg)) as f32
}
