//! Reimplements the arithmetic of the minimap ping functions `0x4eaf00`, `MinimapApplyPing`
//! (`0x4ee330`) and `PingLocation` (`0x4eeca0`). The x87 runs at 53-bit precision, so each chain
//! is `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/minimap_ping.rs` diff
//! each against the exe.

#![allow(clippy::float_cmp)]

/// `1024.0` = `[0x7ffd68]`, the PingLocation projection unit scale.
pub const UNIT1024_BITS: u32 = 0x4480_0000;
/// `8.0` = `[0x807b10]`, the PingLocation final additive bias.
pub const BIAS8_BITS: u32 = 0x4100_0000;
/// `1.0` = `[0x7ff9d8]`, the 0x4ee330 `fdivr` numerator.
pub const ONE_BITS: u32 = 0x3f80_0000;

#[inline]
fn f(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x4eaf00 — four edge values for a bounding-box clamp: lo/hi of an X/Y interpolation pair.
//     `fld [G]; f{sub,add} [w]; fstp [m32]` ×4. `[ebp+0x10]`=x, `[ebp+0xc]`=y, `[ebp+0x14]`=w.
// ===========================================================================================

/// The four independent edge stores of `0x4eaf00` (`0x4eaf24`..`0x4eaf4e`), exactly as ordered:
/// * `[ebp-0x14] = x - w`   (`fld [ebp+0x10]; fsub [ebp+0x14]`)
/// * `[ebp-0x10] = y - w`   (`fld [ebp+0xc];  fsub [ebp+0x14]`)
/// * `[ebp-0xc]  = x + w`   (`fld [ebp+0x10]; fadd [ebp+0x14]`)
/// * `[ebp-0x8]  = y + w`   (`fld [ebp+0xc];  fadd [ebp+0x14]`)
///
/// Each is an independent `fld;f{sub,add};fstp` (no register carry between them). Returns
/// `(x-w, y-w, x+w, y+w)` as f32, the four `[ebp-0x14]`/`[ebp-0x10]`/`[ebp-0xc]`/`[ebp-0x8]` stores.
pub fn bbox_edges(x: f32, y: f32, w: f32) -> (f32, f32, f32, f32) {
    let (xf, yf, wf) = (f64::from(x), f64::from(y), f64::from(w));
    (
        (xf - wf) as f32,
        (yf - wf) as f32,
        (xf + wf) as f32,
        (yf + wf) as f32,
    )
}

// ===========================================================================================
// (2) 0x4ee330 — screen-delta debug log (`%s%f%f`, Player_C.h:357). The owned arithmetic:
//     dx = bx - px ; dy = by - py ; s = 1.0 / (R + R) ; then dx·s and -(s·dy).
//   `R` = the frame-timer `0x6da9b0` st0 result (delegated; consumed UNrounded → f64 input).
// ===========================================================================================

/// `0x4ee330`'s two `fsub` deltas (`0x4ee43a`..`0x4ee456`):
/// * `[ebp-0x8] = bx - px`   (`fld [ebx];   fsub [ebp-0x14]=px`)
/// * `[ebp-0x4] = by - py`   (`fld [ebx+4]; fsub [ebp-0x10]=py`)
///
/// `bx`/`by` are the input C2Vector (`[ebx]`/`[ebx+4]`); `px`/`py` are the vtable
/// `GetPosition`-style result (`[eax]`/`[eax+4]` copied into the frame). Returns `(dx, dy)` f32.
pub fn delta_sub(bx: f32, by: f32, px: f32, py: f32) -> (f32, f32) {
    (
        (f64::from(bx) - f64::from(px)) as f32,
        (f64::from(by) - f64::from(py)) as f32,
    )
}

/// `0x4ee330`'s scale-and-project tail (`0x4ee45b`..`0x4ee482`), with `R` = the live st0 the
/// frame-timer `0x6da9b0` leaves (a PC_53 `f64`, consumed unrounded by `fadd st,st`):
/// * `fadd st,st`        → `R + R`                                              (#3)
/// * `fdivr [0x7ff9d8]`  → `s = 1.0 / (R+R)`                                    (#4)
/// * `fld [ebp-0x8]=dx; fmul st,st(1)` → `dx·s`                                 (#5)
/// * `fxch; fmul [ebp-0x4]=dy`         → `s·dy`                                 (#6)
/// * `fstp [ebp-0x4]` stores `s·dy` as **f32**; `fstp qword [esp+8]` stores `dx·s` as **f64**
/// * `fld [ebp-0x4]; fchs`             → `-(s·dy as f32)` stored as f64         (#7)
///
/// Returns `(sdy_f32, dxs_f64, neg_f64)` = the `[ebp-0x4]` f32 store, the `[esp+8]` f64 store, and
/// the `[esp]` f64 store. `dx`/`dy` are f32 (the prior `fstp`'d deltas); `r` is the f64 timer value.
pub fn scale_project(dx: f32, dy: f32, r: f64) -> (f32, f64, f64) {
    let two_r = r + r; // fadd st,st
    let s = f(ONE_BITS) / two_r; // fdivr [1.0]
    let dxs = f64::from(dx) * s; // fmul st,st(1)  (dx widened, s f64)
    let sdy = s * f64::from(dy); // fmul [ebp-0x4]
    let sdy_f32 = sdy as f32; // fstp dword [ebp-0x4]
    let dxs_f64 = dxs; // fstp qword [esp+8]
    let neg_f64 = -f64::from(sdy_f32); // fld [ebp-0x4](f32); fchs; fstp qword [esp]
    (sdy_f32, dxs_f64, neg_f64)
}

// ===========================================================================================
// (3) 0x4eeca0 'PingLocation' — world→screen projection of a Lua-supplied ping. The owned
//     arithmetic skeleton (delegated st0 producers are inputs; binary constants are mapped):
// ===========================================================================================

/// `#1` (`0x4eecf8`): `m = R + R` stored as f32 (`fstp [ebp-0x10]`). `R` = frame-timer `0x6da9b0`
/// st0 (f64, unrounded). This `m` is the projection multiplier reused by blocks C/D.
pub fn ping_double(r: f64) -> f32 {
    (r + r) as f32
}

/// Blocks A/B `÷1024` legs (`#2,#3` @ `0x4eedd9`/`0x4eede0`; `#5,#6` @ `0x4eee04`/`0x4eee0b`):
/// `fmul [0x7ffd68]=1024; fdivr [c]; fstp [esp]`. `k1` = the `0x41ad70` st0 result (delegated
/// const `1.0`, consumed unrounded → f64 input); `c` = the Lua getter `0x6f3620` f32 store. Yields
/// `arg = c / (k1 · 1024)` as f32 (the arg then pushed to the `0.8·x` helper `0x41ae60`).
pub fn proj_over_unit(c: f32, k1: f64) -> f32 {
    (f64::from(c) / (k1 * f(UNIT1024_BITS))) as f32
}

/// Block A negate (`#4` @ `0x4eedeb`): `fchs; fstp [ebp-0x8]`. `p` = the `0.8·arg` helper
/// (`0x41ae60`) st0 result (f64, unrounded). Yields `X = -(p)` as f32.
pub fn proj_negate(p: f64) -> f32 {
    (-p) as f32
}

/// Blocks C/D `÷V·m` legs (`#7,#8` @ `0x4eee24`/`0x4eee2b`; `#9,#10` @ `0x4eee34`/`0x4eee3a`):
/// `fdivr [n]; fmul [ebp-0x10]=m; fstp [n]`. `n` is the block's numerator (the f32 store from
/// the `0.8·…` helper / the negated X); `v` = the camera vtable getter st0 (f64, unrounded);
/// `m` = `ping_double`'s f32 store. Yields `(n / v) · m` as f32.
pub fn proj_div_scale(n: f32, v: f64, m: f32) -> f32 {
    ((f64::from(n) / v) * f64::from(m)) as f32
}

/// Blocks E offset adds (`#11` @ `0x4eee4e`; `#12` @ `0x4eee61`): `fld [p]; fadd [off]; fstp [p]`.
/// `p` is the projected coordinate (f32 store); `off` = the screen-anchor component from the
/// `[edx+0x14]` vtable out-param (f32). Yields `p + off` as f32.
pub fn proj_add_offset(p: f32, off: f32) -> f32 {
    (f64::from(p) + f64::from(off)) as f32
}

/// Block F final bias (`#13` @ `0x4eeee9`): `fld [ebp-0x10]=z; fadd [0x807b10]=8; fstp [ebp-0x40]`.
/// `z` is the frame slot (re-zeroed to `0.0` at `0x4eeec8` on this path). Yields `z + 8.0` as f32.
pub fn proj_bias8(z: f32) -> f32 {
    (f64::from(z) + f(BIAS8_BITS)) as f32
}
