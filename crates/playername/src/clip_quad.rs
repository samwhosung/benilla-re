//! `0x6c7cc0`'s floating-point slice — the **label-plate screen-rect clip + quad build**.
//!
//! `0x6c7cc0` (`PlayerName.cpp`, the clip-quad plate render) projects a WORLDTEXTSTRING's world
//! anchor to screen, clamps the screen center into the visible viewport rectangle (so the plate
//! never half-leaves the screen), then builds the four quad corners and writes the plate's NDC
//! anchor into `this[0x30/0x34/0x38]`. The world→screen projection (`0x483ee0`), the viewport
//! fetch (`0x768320`), the rising-Z interp offset (`0x6c8070`, a sibling primitive), and a gx state
//! call (`0x509520`) are delegated calls — not transcribed here; their results are fed. This module
//! transcribes EXACTLY the arithmetic in `[0x6c7da8, 0x6c7ef9)`: the **screen-rect clamp**
//! and the **quad-corner + midpoint build**.
//!
//! ## Inputs (all f32 unless noted), from `this` (`esi`) and the fed results of the delegated calls:
//! - `cx = this[0xc]` — plate half-width (px). `cy = this[0x10]` — plate half-height (px).
//! - `sx`/`sy` — the projected screen-x/y (`0x483ee0` out, `[ebp-0x10]`/`[ebp-0xc]`).
//! - `add_x`/`add_y` — the viewport-origin addends the projected x/y receive (`[ebp-0x54]` to x,
//!   `[ebp-0x58]` to y; from the `0x768320` viewport fetch). So `x = sx + add_x`, `y = sy + add_y`.
//! - `vp_half_w = [0x832a44]`, `vp_half_h = [0x832a48]` — the viewport half-extents the `0x41ae40/
//!   50/60/70` converters read. `0x41ae60(1.0) = vp_half_w` (the X-clamp upper base), `0x41ae70(1.0)
//!   = vp_half_h` (Y). `0x41ae40(t) = t / vp_half_w`, `0x41ae50(t) = t / vp_half_h` (px→NDC).
//!
//! ## The op trace (every FP-arith op in `[0x6c7da8, 0x6c7ef9)`; load/store/compare excluded):
//! ```text
//!   6c7dae fadd [ebp-0x54]   x = sx + add_x
//!   6c7dc0 fadd [ebp-0x58]   y = sy + add_y
//!   --- X clamp into [cx, vp_half_w − cx] ---
//!   6c7dc6 fcomp [ebp+8]=cx                 ; m = max(x, cx)
//!   6c7de6 call 0x41ae60(1.0) = vp_half_w
//!   6c7deb fsub [ebp+8]=cx                  ; vp_half_w − cx
//!   6c7dee fcomp m                          ; if (vp_half_w−cx) <= m → upper-clamp
//!   6c7e1a call 0x41ae60(1.0); 6c7e1f fsub cx ; clamped_x = vp_half_w − cx   (upper branch)
//!                                            ; else clamped_x = clamp(x, cx, …) = max(x,cx) capped
//!   --- Y clamp into [cy, vp_half_h − cy] ---  (mirror; result left live on st0)
//!   6c7e45 call 0x41ae70(1.0) = vp_half_h
//!   6c7e4a fsub [ebp-4]=cy                  ; vp_half_h − cy
//!   6c7e73 call 0x41ae70(1.0); 6c7e78 fsub cy ; (upper branch)
//!   --- quad corners from clamped center (clamped_y live on st0) ---
//!   6c7e7b fld st(0)         ; dup clamped_y (80-bit, no f32 reload)
//!   6c7e80 fadd [esi+0x10]=cy  → top    = clamped_y + cy   → local_38
//!   6c7e8e fsub [esi+0xc]=cx   → left   = clamped_x − cx   → local_34
//!   6c7e94 fsub [esi+0x10]=cy  → bottom = clamped_y − cy   → local_30
//!   6c7e9d fadd [esi+0xc]=cx   → right  = clamped_x + cx   → local_2c
//!   --- midpoints → px→NDC → write this[0x30/0x34/0x38] ---
//!   6c7eab fadd local_38; 6c7eaf fmul [0x7ffa24]=0.5 ; midY = (bottom+top)·0.5
//!   6c7eb8 call 0x41ae50(midY) = midY / vp_half_h     ; this[0x34] = midY_ndc
//!   6c7ec4 fadd local_34; 6c7ec7 fmul [0x7ffa24]=0.5 ; midX = (right+left)·0.5
//!   6c7ed0 call 0x41ae40(midX) = midX / vp_half_w
//!   6c7ed5 fsub [esi+0xc]=cx                          ; this[0x30] = midX_ndc − cx
//!                                                       this[0x38] = 0
//! ```
//! FP-arith count = 16 (the `fadd`/`fsub`/`fmul`/`fdiv` ops; the `0x41ae40/50/60/70` each contribute
//! one `fdiv`/`fmul`, counted at the call site). The `fcomp`/`fld st(0)` dup and the loads/stores
//! are not in the arith set but ride the executed chain.
//!
//! **Precision.** Every x87 op rounds to 53-bit = IEEE
//! `f64`; an `f32` memory operand widens exactly via `f64::from(f32::from_bits(..))`; `as f32` is
//! written at exactly each `fstp dword [m32]`. The Y-clamp result stays live on st0 into the
//! `fld st(0)` dup, so the quad's top and bottom are built from the unrounded clamped Y, never an
//! `f32` reload. The `0x41ae40/50/60/70` converters round their `fdiv`/`fmul`
//! result to f32 at the caller's `fstp dword`, so each converter output is `as f32`.
#![allow(clippy::float_cmp)]

/// `[0x7ffa24]` = 0.5 (`0x3f000000`), the midpoint scale (`fmul` before each px→NDC convert).
pub const HALF_BITS: u32 = 0x3f00_0000;
/// `[0x832a44]` = the viewport half-width divisor/multiplier (`0x41ae40` `fdiv`, `0x41ae60` `fmul`).
/// Static `.data` default `0x3f4ccccd` (0.8); the real value is the live viewport half-width.
pub const VP_HALF_W_DEFAULT_BITS: u32 = 0x3f4c_cccd;
/// `[0x832a48]` = the viewport half-height divisor/multiplier (`0x41ae50` `fdiv`, `0x41ae70` `fmul`).
/// Static `.data` default `0x3f19999a` (0.6); the real value is the live viewport half-height.
pub const VP_HALF_H_DEFAULT_BITS: u32 = 0x3f19_999a;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits)) // an f32 memory operand widens exactly to the PC_53 f64 chain
}

// The `0x41ae40/50/60/70` converters each leave their `fdiv`/`fmul` result live on st0 (80-bit /
// PC_53 f64) — the caller operates on that f64 and narrows to f32 only at its own explicit `fstp
// dword`. So these return f64; each call site below applies `as f32` exactly where the binary stores.

/// `0x41ae40(t) = t / vp_half_w` (px→NDC, X axis); result left on st0 as f64.
#[inline]
fn ae40(t: f64, vp_half_w: f32) -> f64 {
    t / k(vp_half_w.to_bits())
}
/// `0x41ae50(t) = t / vp_half_h` (px→NDC, Y axis); result left on st0 as f64.
#[inline]
fn ae50(t: f64, vp_half_h: f32) -> f64 {
    t / k(vp_half_h.to_bits())
}
/// `0x41ae60(t) = vp_half_w · t`; result left on st0 as f64. `0x41ae60(1.0) = vp_half_w`.
#[inline]
fn ae60(t: f64, vp_half_w: f32) -> f64 {
    k(vp_half_w.to_bits()) * t
}
/// `0x41ae70(t) = vp_half_h · t`; result left on st0 as f64. `0x41ae70(1.0) = vp_half_h`.
#[inline]
fn ae70(t: f64, vp_half_h: f32) -> f64 {
    k(vp_half_h.to_bits()) * t
}

/// The clamp+quad result of `0x6c7cc0`'s `[0x6c7da8, 0x6c7ef9)` slice: the three plate-anchor
/// fields written to `this[0x30/0x34/0x38]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlateAnchor {
    /// `this[0x30]` = `midX_ndc − cx`.
    pub anchor_x: u32,
    /// `this[0x34]` = `midY_ndc`.
    pub anchor_y: u32,
    /// `this[0x38]` = 0.
    pub anchor_z: u32,
}

/// Faithful transcription of `0x6c7cc0`'s screen-rect clamp + quad-corner + midpoint build.
///
/// `sx`/`sy` are the projected screen x/y (pre origin-add); `add_x`/`add_y` the viewport-origin
/// addends; `cx`/`cy` the plate half-extents (`this[0xc]`/`this[0x10]`); `vp_half_w`/`vp_half_h`
/// the viewport half-extents (`[0x832a44]`/`[0x832a48]`). Returns the three NDC plate-anchor fields
/// exactly as the binary writes them (bit-exact `u32` payloads), built at PC_53 f64 with `as f32`
/// at each `fstp dword`.
#[allow(clippy::too_many_arguments)]
pub fn clip_quad(
    cx: f32,
    cy: f32,
    sx: f32,
    sy: f32,
    add_x: f32,
    add_y: f32,
    vp_half_w: f32,
    vp_half_h: f32,
) -> PlateAnchor {
    // 6c7dae / 6c7dc0: x = sx + add_x ; y = sy + add_y  (both stored back as f32).
    let x = (k(sx.to_bits()) + k(add_x.to_bits())) as f32;
    let y = (k(sy.to_bits()) + k(add_y.to_bits())) as f32;
    let cx_f = k(cx.to_bits());
    let cy_f = k(cy.to_bits());

    // ---- X clamp into [cx, vp_half_w − cx] ----
    // 6c7dc6: m = (x <= cx) ? cx : x   (fcomp x,cx ; jne taken when x<cx or x==cx → pick cx)
    let m_x = if k(x.to_bits()) <= cx_f {
        cx
    } else {
        x // x > cx
    };
    // 6c7de6..6c7df1: upper = vp_half_w − cx (live st0 f64, compared by fcomp, not narrowed).
    let upper_x: f64 = ae60(1.0, vp_half_w) - cx_f;
    let clamped_x = if upper_x <= k(m_x.to_bits()) {
        // 6c7e1a..6c7e22: clamped_x = (vp_half_w − cx) narrowed to f32 at fstp [ebp+8].
        (ae60(1.0, vp_half_w) - cx_f) as f32
    } else if k(x.to_bits()) <= cx_f {
        // 6c7df8: inner — (x <= cx) ? cx : x.
        cx // x <= cx → cx (lower)
    } else {
        x // x > cx → x
    };

    // ---- Y clamp into [cy, vp_half_h − cy] ----  (result kept as f64 = live st0)
    // 6c7e25: m = (y <= cy) ? cy : y.
    let m_y = if k(y.to_bits()) <= cy_f { cy } else { y };
    // 6c7e45..6c7e52: upper = vp_half_h − cy (live st0 f64, compared by fcomp, not narrowed).
    let upper_y: f64 = ae70(1.0, vp_half_h) - cy_f;
    let clamped_y: f64 = if upper_y <= k(m_y.to_bits()) {
        // 6c7e6e..6c7e78: vp_half_h − cy, NOT stored to f32 — pushed onto st0 and left live (f64).
        ae70(1.0, vp_half_h) - cy_f
    } else if k(y.to_bits()) <= cy_f {
        // 6c7e57: inner — (y <= cy) ? cy : y, each `fld dword` (so an f32-widened value on st0).
        cy_f // 6c7e69: fld [ebp-4] = cy
    } else {
        k(y.to_bits()) // 6c7e64: fld [ebp-0xc] = y
    };

    // ---- quad corners (clamped_y is the live st0; `fld st(0)` dup keeps 80-bit, no f32 reload) ----
    let top = (clamped_y + cy_f) as f32; // 6c7e80 fadd cy ; 6c7e88 fstp local_38
    let left = (k(clamped_x.to_bits()) - cx_f) as f32; // 6c7e8e fsub cx ; fstp local_34
    let bottom = (clamped_y - cy_f) as f32; // 6c7e94 fsub cy (st0 still clamped_y) ; fstp local_30
    let right = (k(clamped_x.to_bits()) + cx_f) as f32; // 6c7e9d fadd cx ; fstp local_2c

    // ---- midpoints → px→NDC → anchor fields ----
    // 6c7ea8..6c7eb5: midY = (bottom + top) · 0.5 ; 6c7ecd fstp [esp] narrows midY to f32 (the
    // converter arg) ; 6c7eb8 ae50 ; 6c7ebd fstp [ebp+8] narrows the NDC-Y to f32 → this[0x34].
    let mid_y = ((k(bottom.to_bits()) + k(top.to_bits())) * k(HALF_BITS)) as f32; // fstp [esp]
    let anchor_y = ae50(k(mid_y.to_bits()), vp_half_h) as f32; // fstp [ebp+8]
                                                               // 6c7ec1..6c7ecd: midX = (right + left) · 0.5 (narrowed to f32 at fstp [esp]) ; 6c7ed0 ae40
                                                               // (result live st0 f64) ; 6c7ed5 fsub cx (live f64) ; 6c7ede fstp → this[0x30] = (midX_ndc − cx).
    let mid_x = ((k(right.to_bits()) + k(left.to_bits())) * k(HALF_BITS)) as f32; // fstp [esp]
    let anchor_x = (ae40(k(mid_x.to_bits()), vp_half_w) - cx_f) as f32;

    PlateAnchor {
        anchor_x: anchor_x.to_bits(),
        anchor_y: anchor_y.to_bits(),
        anchor_z: 0u32, // 6c7ede mov [ebp-0x14],0 → this[0x38] = 0
    }
}
