//! Reimplements the vector kernels `AddVector3InPlace` (`0x4841c0`), `SubtractVector3`
//! (`0x4841f0`), and the normalise-and-range-gate function `0x4813b0`. The x87 runs at 53-bit
//! precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in
//! `tests/vector3_ops.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `[0x7ffd74]` = 0.0 (`0x00000000`), the 0x4813b0 lower range bound.
pub const LO_BITS: u32 = 0x0000_0000;
/// `[0x7ff9d8]` = 1.0 (`0x3f800000`), the 0x4813b0 upper range bound.
pub const HI_BITS: u32 = 0x3f80_0000;

// ===========================================================================================
// (1) 0x4813b0 — 2× normalize, range gates, then vec3 += vec3 (×2).
// ===========================================================================================

/// The two normalize divisions of `0x4813b0` (`0x4813b6`..`0x4813ed`):
/// * `[ebp-4] = (arg0 − lo_x) / (hi_x − lo_x)`  (`fld arg0; fsub lo_x; fld hi_x; fsub lo_x; fdivp`)
/// * `[ebp+8] = (arg1 − lo_y) / (hi_y − lo_y)`
///
/// where `arg0 = [ebp+8]`, `arg1 = [ebp+0xc]`, and the four `lo/hi` are object fields
/// `[ebx+0x394]` (lo_x), `[ebx+0x39c]` (hi_x), `[ebx+0x390]` (lo_y), `[ebx+0x398]` (hi_y). Each
/// numerator/denominator is a live PC_53 `f64`; `fdivp` divides numerator by denominator and the
/// `fstp` narrows to `f32`. Returns `(coord_x, coord_y)` as the two stored f32s.
pub fn normalize_pair(
    arg0: f32,
    arg1: f32,
    lo_x: f32,
    hi_x: f32,
    lo_y: f32,
    hi_y: f32,
) -> (f32, f32) {
    let cx = (f64::from(arg0) - f64::from(lo_x)) / (f64::from(hi_x) - f64::from(lo_x));
    let cy = (f64::from(arg1) - f64::from(lo_y)) / (f64::from(hi_y) - f64::from(lo_y));
    (cx as f32, cy as f32)
}

/// The four range gates of `0x4813b0` (`0x4813ed`..`0x481439`): after the two normalized coords are
/// computed, each is gated against `[0.0, 1.0]` and the function proceeds to the vec-add only if
/// BOTH coords lie in range; otherwise it returns 0. The exact x87 branch reconstruction (the
/// boundary is `fcomp` against the loaded constant, read via `fnstsw ah`):
/// * gate1/2 (`fcomp 0.0; test ah,5; jnp out`): `jnp` taken ⇔ ODD parity of `{C0,C2}` ⇔ ordered
///   `coord < 0.0` ⇒ out. So a coord PASSES iff `coord >= 0.0`.
/// * gate3/4 (`fcomp 1.0; test ah,0x41; je out`): `je` taken ⇔ `(ah & 0x41)==0` ⇔ `C0=0 & C3=0` ⇔
///   `coord > 1.0` ⇒ out. So a coord PASSES iff `coord <= 1.0`.
///
/// A NaN coord: gate1 (`fcomp 0.0`) sets C0=C2=C3=1 ⇒ `ah&5 = 5` ⇒ even parity ⇒ `jnp` NOT taken
/// (survives); gate3 (`fcomp 1.0`) sets C0=1 ⇒ `ah&0x41` ≠ 0 ⇒ `je` NOT taken (survives). So a NaN
/// coord survives ALL four gates in the binary — this fn returns the binary's verdict exactly,
/// hence the explicit `<`/`>` ordered comparisons (a NaN is never `< 0.0` nor `> 1.0`).
///
/// Returns `true` iff control reaches the vec-add (binary halts at `0x481439`), `false` iff it
/// branches to the `xor eax,eax` exit (`0x4814ac`). Faithfully replicates each emitted `jcc`.
pub fn in_range(coord_x: f32, coord_y: f32) -> bool {
    // gate1: jnp out  ⇔ ordered (coord_x < 0.0). PASS ⇔ NOT(coord_x < 0.0).
    if coord_x < 0.0 {
        return false;
    }
    // gate2: same on coord_y.
    if coord_y < 0.0 {
        return false;
    }
    // gate3: je out ⇔ (coord_x > 1.0). PASS ⇔ NOT(coord_x > 1.0).
    if coord_x > 1.0 {
        return false;
    }
    // gate4: same on coord_y.
    if coord_y > 1.0 {
        return false;
    }
    true
}

/// The vec3 in-place add shape repeated 6× in `0x4813b0`'s in-range body (the `[edi]` block at
/// `0x481466`..`0x481489` and the `[esi]` block at `0x48148b`..`0x4814a4`): a source vec3 `s`
/// (loaded from the object graph `[[ebx+0x65b8]+8]`) is added componentwise into a destination
/// vec3 `d` in place. Each component is `fld [s.i]; fadd [d.i]; fstp [d.i]` — a live PC_53 sum
/// narrowed to `f32` at the store. Returns the updated destination `(d.x, d.y, d.z)`.
pub fn vec3_add_inplace(d: [f32; 3], s: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(s[0]) + f64::from(d[0])) as f32,
        (f64::from(s[1]) + f64::from(d[1])) as f32,
        (f64::from(s[2]) + f64::from(d[2])) as f32,
    ]
}

// ===========================================================================================
// (3) 0x4841c0 — vec3 dst += src.
// ===========================================================================================

/// `0x4841c0` (`0x4841c8`..`0x4841dd`, `ret 4`): `eax = ecx` (dst = `this`), `ecx = [ebp+8]` (src
/// arg), then componentwise `fld [src.i]; fadd [dst.i]; fstp [dst.i]` for i = 0,1,2 — so
/// `dst.xyz += src.xyz`, stored in place into `dst`. Each component is a live PC_53 sum narrowed to
/// `f32` at its `fstp`. `dst`/`src` are the two vec3s. Returns the updated `dst = (x,y,z)`.
pub fn vec3_add(dst: [f32; 3], src: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(src[0]) + f64::from(dst[0])) as f32,
        (f64::from(src[1]) + f64::from(dst[1])) as f32,
        (f64::from(src[2]) + f64::from(dst[2])) as f32,
    ]
}

// ===========================================================================================
// (4) 0x4841f0 — vec3 dst = minuend − subtrahend.
// ===========================================================================================

/// `0x4841f0` (`0x4841f3`..`0x48420d`, `ret 4`): `eax = ecx` (dst = `this`), `ecx = [ebp+8]`
/// (subtrahend arg), `edx` the minuend pointer. The three `fld [minuend.i]; fsub [subtrahend.i]`
/// load st0/st1/st2 = `(m.z−s.z, m.y−s.y, m.x−s.x)` (loaded z,y,x), then three `fstp` store in FILO
/// order to `dst.x, dst.y, dst.z` — so `dst[i] = minuend[i] − subtrahend[i]` for i = 0,1,2. Each
/// difference is a live PC_53 value narrowed to `f32` at its `fstp`. Returns `dst = (x,y,z)`.
pub fn vec3_sub(minuend: [f32; 3], subtrahend: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(minuend[0]) - f64::from(subtrahend[0])) as f32,
        (f64::from(minuend[1]) - f64::from(subtrahend[1])) as f32,
        (f64::from(minuend[2]) - f64::from(subtrahend[2])) as f32,
    ]
}
