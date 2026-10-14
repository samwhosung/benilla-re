//! Reimplements the arithmetic of the minimap blip projection and render functions `0x4ebc00`,
//! `0x4ebf20`, `0x4ec090`, `0x4ec440` and `MinimapFrameRender` (`0x4ec950`). The x87 runs at
//! 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests
//! in `tests/minimap_blips.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact). The `0xbc8xxx`/`0xbc79xx` operands are
// RUNTIME globals (.data/.bss matrix + state), supplied as inputs by the difftest, not constants.
// ===========================================================================================

/// 1.0 (`DAT_007ff9d8`), the perspective-divide numerator.
pub const ONE_BITS: u32 = 0x3f80_0000;
/// 0.5 (`DAT_007ffa24`), the NDC→screen affine bias / the 0.5 scales / `h = 0.5·center`.
pub const HALF_BITS: u32 = 0x3f00_0000;
/// 1/3 = 0.33333334 (`DAT_0084c534`), the NDC→screen affine scale.
pub const THIRD_BITS: u32 = 0x3eaa_aaab;
/// 0.8 (`DAT_00807a24`), the rotation radius factor (`r = 0.8·radius`).
pub const ZP8_BITS: u32 = 0x3f4c_cccd;

#[inline]
fn one() -> f64 {
    f64::from(f32::from_bits(ONE_BITS))
}
#[inline]
fn half() -> f64 {
    f64::from(f32::from_bits(HALF_BITS))
}
#[inline]
fn third() -> f64 {
    f64::from(f32::from_bits(THIRD_BITS))
}
#[inline]
fn zp8() -> f64 {
    f64::from(f32::from_bits(ZP8_BITS))
}

// ===========================================================================================
// (1) 0x4ebc00 — per-row color/gradient 3×3 accumulate (loop body 0x4ebd72..0x4ebda7)
// ===========================================================================================

/// One iteration's owned arithmetic of `0x4ebc00`'s gradient loop (`0x4ebd72`..`0x4ebda7`). The
/// stack holds three loop-invariant values `A = *[ebp-0x8]`, `B = [ebp-0x20]`, `C = [ebp-0x1c]`
/// (pushed once before the inner loop, st0=C, st1=B, st2=A). Each iteration multiplies the SAME
/// scalar `A` (read off `st(2)/st(3)/st(4)` — all `A`) by three gradient-matrix columns indexed by
/// the per-iteration `eax` step (`[eax+0xbc8230]`, `[eax+0xbc8228]`, `[eax+0xbc822c]`), then folds:
/// * `p30 = A·m30` (`fmul [+0xbc8230]`), `p28 = A·m28` (`fmul [+0xbc8228]`), `p2c = A·m2c`
///   (`fmul [+0xbc822c]`).
/// * `out0 = B + p30` (`fld st(3)=B; fadd st,st(2)=p30`) → `fstp [ebp-0x2c]` (then `mov [edx]`).
/// * `out1 = C + p28` (`fld st(2)=C; fadd st,st(1)=p28`) → `fstp [ebp-0x28]` (then `mov [edx+4]`).
/// * `out2 = p2c` (`fstp [ebp-0x30]`, reloaded `fld; fstp [ebp-0x24]`) → `mov [edx+8]`.
///
/// `B` is pushed but participates only as the `out0` offset; the products all use `A`. Adds fold
/// register-LIVE PC_53 partials (`B`/`C` widen f32→f64; `A·m` stays live). Each output `fstp`'d →
/// f32. `a`/`b`/`c` are A/B/C; `m30`/`m28`/`m2c` the three matrix columns for this `eax` step.
/// Returns `(out0_f32, out1_f32, out2_f32)` = `[ebp-0x2c]`/`[ebp-0x28]`/`[ebp-0x24]`.
pub fn gradient_row(a: f32, b: f32, c: f32, m30: f32, m28: f32, m2c: f32) -> (f32, f32, f32) {
    let av = f64::from(a);
    let p30 = av * f64::from(m30);
    let p28 = av * f64::from(m28);
    let p2c = av * f64::from(m2c);
    let out0 = f64::from(b) + p30; // fadd st,st(2): B (f32→f64) + live p30
    let out1 = f64::from(c) + p28; // fadd st,st(1): C (f32→f64) + live p28
    (out0 as f32, out1 as f32, p2c as f32)
}

// ===========================================================================================
// (2) 0x4ebf20 — zoom-span reciprocal pair (0x4ebfca..0x4ec01b)
// ===========================================================================================

/// `0x4ebf20`'s owned arithmetic (`0x4ebfca`..`0x4ec012`): an integer span (loaded via
/// `fild qword` with the high dword forced to 0 — the **zero-extended u32** read as an i64) is
/// divided INTO the constant `[0x7ffa24]=0.5` (`fdivr` = `mem / st0` → `0.5 / span`), the quotient
/// `fstp`'d → f32, then `1.0 − quotient` (`fld 1.0; fsub [quot]`) `fstp`'d → f32.
/// Returns `(recip_f32, complement_f32)`.
pub fn zoom_span(span: u32) -> (f32, f32) {
    let s = i64::from(span) as f64; // fild qword {span, 0}
    let recip = (half() / s) as f32; // fdivr [0x7ffa24]=0.5; fstp → f32
    let complement = (one() - f64::from(recip)) as f32; // fld 1.0; fsub [recip]; fstp → f32
    (recip, complement)
}

// ===========================================================================================
// (3) 0x4ec090 — percent-scale + negate (0x4ec130..0x4ec159, 0x4ec3bb..0x4ec3d8)
// ===========================================================================================

/// `0x4ec090`'s percent-scale (`0x4ec130`..`0x4ec159`): a global `g=[0xbc830c]` times `k=[0x80308c]`,
/// `fst`'d (NO pop) to `[ebp-0x4]` then `fchs`'d and `fstp`'d to `[ebp-0x8]`. `[ebp-0x4]` holds
/// `g·k` (f32); `[ebp-0x8]` holds `−(g·k)` (the negate of the live st0 = that same f32, exact).
/// Returns `(scaled_f32, negated_f32)`.
pub fn percent_scale(g: f32, k: f32) -> (f32, f32) {
    let v = (f64::from(g) * f64::from(k)) as f32;
    (v, -v)
}

/// `0x4ec090`'s 3× component negate (`0x4ec3bb`..`0x4ec3d8`): three values from
/// `[ebp-0x20]/[ebp-0x1c]/[ebp-0x18]` each `fchs`'d and `fstp`'d → f32. Exact sign flip.
pub fn negate3(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    (
        (-f64::from(x)) as f32,
        (-f64::from(y)) as f32,
        (-f64::from(z)) as f32,
    )
}

// ===========================================================================================
// (4) 0x4ec440 — 0.5-scale + reciprocal·scale (0x4ec4d0..0x4ec6f4)
// ===========================================================================================

/// `0x4ec440`'s leading scale (`0x4ec4d0`..`0x4ec4fa`): `[ebp+8]·0.5` (`fmul [0x7ffa24]`), `fst`'d
/// (NO pop) to `[ebp+8]` then `fchs`'d and `fstp`'d to `[ebp-0x10]`. Like [`percent_scale`].
/// Returns `(scaled_f32, negated_f32)`.
pub fn zoom_scale(arg: f32) -> (f32, f32) {
    let v = (f64::from(arg) * half()) as f32;
    (v, -v)
}

/// `0x4ec440`'s reciprocal·scale (`0x4ec60c`..`0x4ec629`): `fld 1.0; fdiv [ebp+8]` → `1.0/arg`,
/// `fst`'d (NO pop) to `[ebp+0xc]`, then `fmul [0x807a40]=k` on the LIVE PC_53 reciprocal and
/// `fst`/`fst`/`fstp`'d into a vec3. Returns `(recip_f32, scaled_f32)`.
pub fn recip_scale(arg: f32, k: f32) -> (f32, f32) {
    let inv = one() / f64::from(arg);
    let recip_f32 = inv as f32;
    let scaled = (inv * f64::from(k)) as f32;
    (recip_f32, scaled)
}

/// `0x4ec440`'s tail 0.5-scale (`0x4ec6dd`..`0x4ec6f4`): `[ebp+0xc]·0.5` (`fmul [0x7ffa24]`),
/// broadcast `fst`/`fst`/`fstp`'d → f32 into a vec3. Returns the f32 product.
pub fn half_scale(v: f32) -> f32 {
    (f64::from(v) * half()) as f32
}

// ===========================================================================================
// (5) 0x4ec950 — LARGE render/pick driver. Slice A: unproject/pick transform.
// ===========================================================================================

/// Slice A, the 3-axis delta (`0x4ecf04`..`0x4ecf31`): `d = far − near` per axis, each
/// `fld [far]; fsub [near]` left LIVE on the x87 stack (NOT stored — fed directly into the matrix
/// multiply). Returns the three deltas at PC_53 f64.
pub fn delta3(far: [f32; 3], near: [f32; 3]) -> [f64; 3] {
    [
        f64::from(far[0]) - f64::from(near[0]),
        f64::from(far[1]) - f64::from(near[1]),
        f64::from(far[2]) - f64::from(near[2]),
    ]
}

/// Slice A, the first 3×3 matrix transform `outA = M·d` (`0x4ecf37`..`0x4ecf94`). `d` is the live
/// [`delta3`]. Each output folds newest-term-first, matching the `faddp` sequence:
/// `outA_i = (d[2]·c[2] + d[1]·c[1]) + d[0]·c[0]` for column `c = rows[i]`.
/// `outA0` → `fstp [ebp-0x58]` (f32), `outA1` → `fstp [ebp-0x54]` (f32), `outA2` left LIVE (f64).
/// Returns `(outA0_f32, outA1_f32, outA2_live_f64)`.
pub fn mat_m(d: [f64; 3], rows: [[f32; 3]; 3]) -> (f32, f32, f64) {
    let dot = |c: [f32; 3]| -> f64 {
        ((d[2] * f64::from(c[2])) + (d[1] * f64::from(c[1]))) + (d[0] * f64::from(c[0]))
    };
    (dot(rows[0]) as f32, dot(rows[1]) as f32, dot(rows[2]))
}

/// Slice A, the second matrix transform `(px, py) = N·outA` (`0x4ecf96`..`0x4ecfd2`). Inputs:
/// `outA0` (reloaded f32 from `[ebp-0x58]`), `outA1` (reloaded f32 from `[ebp-0x54]`), `outA2`
/// (the LIVE f64 from [`mat_m`], NOT reloaded). Each folds newest-first:
/// `px = (a2·c[2] + a1·c[1]) + a0·c[0]` for `rows[0]` — left LIVE on st0 (→ persp);
/// `py = (a2·c[2] + a1·c[1]) + a0·c[0]` for `rows[1]` — `fstp [ebp-0x98]` → f32.
/// Returns `(px_live_f64, py_f32)`.
pub fn mat_n(out_a: (f32, f32, f64), rows: [[f32; 3]; 2]) -> (f64, f32) {
    let a0 = f64::from(out_a.0);
    let a1 = f64::from(out_a.1);
    let a2 = out_a.2; // LIVE f64 — outA2 is never spilled to f32.
    let dot = |c: [f32; 3]| -> f64 {
        ((a2 * f64::from(c[2])) + (a1 * f64::from(c[1]))) + (a0 * f64::from(c[0]))
    };
    (dot(rows[0]), dot(rows[1]) as f32)
}

/// Slice A, perspective divide + NDC→screen affine for X (`0x4ecfd8`..`0x4ecff5`): `invD =
/// 1.0/[depth]`, `fst [ebp-0x4]` (NO pop — st0 stays LIVE), then `screenX = ((invD_LIVE · px)·
/// (1/3)) + 0.5`. invD is the un-rounded PC_53 reciprocal (LIVE st0). `px` is the live f64 from
/// [`mat_n`]. Returns `screenX` as f32.
pub fn persp_map_x(depth: f32, px: f64) -> f32 {
    let inv_d = one() / f64::from(depth);
    (((inv_d * px) * third()) + half()) as f32
}

/// Slice A, perspective divide + affine for Y (`0x4ecffc`..`0x4ed011`): unlike X, this RELOADS
/// `invD` as f32 (`fld [ebp-0x4]`), so `screenY = ((invD_F32 · py)·(1/3)) + 0.5`. The f32 reload
/// is live-vs-reload distinction. `py` is the f32 from [`mat_n`].
pub fn persp_map_y(depth: f32, py: f32) -> f32 {
    let inv_d_f32 = (one() / f64::from(depth)) as f32;
    (((f64::from(inv_d_f32) * f64::from(py)) * third()) + half()) as f32
}

// ===========================================================================================
// (6) 0x4ec950 — Slice B: per-element scale-and-offset loop (0x4ed1c0..0x4ed21d)
// ===========================================================================================

/// Slice B, one iteration of the blip scale loop (`0x4ed1c0`..`0x4ed214`): a per-node scale
/// `S = [ebx+0x7c]` times a 3-vector `(m0, m1, m2) = ([eax-8], [eax-4], [eax-0xc])`, plus screen
/// offsets `(offX, offY) = ([ebp-0x54], [ebp-0x50])`:
/// `outX = (S_LIVE·m0) + offX` (S·m0 stays LIVE PC_53 across `fadd [offX]`),
/// `outY = f32(S·m1) + offY` (S·m1 `fstp`'d → f32 then reloaded before `+offY`),
/// `outZ = f32(S·m2)`. Each output `fstp`'d → f32.
pub fn scale_offset(s: f32, m: [f32; 3], off_x: f32, off_y: f32) -> [f32; 3] {
    let s = f64::from(s);
    let out_x = ((s * f64::from(m[0])) + f64::from(off_x)) as f32;
    let s_m1 = (s * f64::from(m[1])) as f32;
    let out_y = (f64::from(s_m1) + f64::from(off_y)) as f32;
    let out_z = (s * f64::from(m[2])) as f32;
    [out_x, out_y, out_z]
}

// ===========================================================================================
// (7) 0x4ec950 — Slices C/D: fsincos compass/blip rotation (0x4ed452.. / 0x4ed4f7..)
// ===========================================================================================

/// Slices C and D, the `fsincos` rotation kernel (`0x4ed452`..`0x4ed495`; D `0x4ed4f7`..`0x4ed539`
/// is the instruction-twin). Given angle `theta`, radius global `g_radius` (`[0xbc8274]`), center
/// global `g_center` (`[0xbc7970]`), and a flag selecting which trig value is the X-term:
/// * `fsincos(theta)` → `cos, sin`, each `fstp`'d → f32 then RELOADED for the products.
/// * `r = 0.8 · g_radius` (LIVE f64); `h = 0.5 · g_center` (LIVE f64).
/// * `outX = h − r·u_f32` (`fmul [slotA]; fchs; fadd st,st(1)=h`).
/// * `outY = h + r·v_f32` (`fmul [slotB]; fadd st,st(1)=h`).
/// * `outZ = 0` (set by `mov`, not FP).
///
/// **C and D differ ONLY in which trig value lands in each `[ebp-?]` slot:** C stores
/// `cos→[ebp-0x4]`, `sin→[ebp-0xc]`, so `(u,v) = (cos, sin)`; D swaps the entry `lea`s
/// (`cos→[ebp-0xc]`, `sin→[ebp-0x4]`), so `(u,v) = (sin, cos)`. The FP-arith bytes are identical;
/// the swap is in the integer entry glue. `x_is_cos = true` ⇒ C, `false` ⇒ D.
///
/// `fsincos` routes through host libm in both the emulator and Rust — the
/// transcendental is diffed structurally; the surrounding `fmul/fchs/fadd` is owned bit-exact.
/// `u`/`v` enter as f32 (spill+reload); `r` and `h` stay PC_53. Each output `fstp`'d → f32.
pub fn rotate(theta: f32, g_radius: f32, g_center: f32, x_is_cos: bool) -> (f32, f32, f32) {
    let t = f64::from(theta);
    let cos_f32 = t.cos() as f32; // fstp → f32
    let sin_f32 = t.sin() as f32; // fstp → f32
    let (u, v) = if x_is_cos {
        (cos_f32, sin_f32) // Slice C
    } else {
        (sin_f32, cos_f32) // Slice D
    };
    let r = zp8() * f64::from(g_radius); // LIVE
    let h = f64::from(g_center) * half(); // LIVE
    let out_x = ((-(r * f64::from(u))) + h) as f32;
    let out_y = ((r * f64::from(v)) + h) as f32;
    (out_x, out_y, 0.0)
}
