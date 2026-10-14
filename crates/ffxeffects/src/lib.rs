//! The CPU maths of the full-frame post-process effects, `FFXEffects.cpp` (the glow, death and
//! rectangle passes, `0x6cb020` to `0x6cdb40`): the colour packs, the quad UVs and corners, the
//! render-target dimension chain and the glow phase and scroll. The per-pixel blur, glow and death
//! convolution runs in the graphics layer's fragment programs and is not here.
//!
//! The glow-wave table generator (`0x6cbea0`) calls the x87 `fsin`, which differs between CPUs; the
//! functions here take its result as an input and pack it.

// The table packers keep the exe's two comparisons in its order (`v < lo` gives `lo`, then
// `v >= hi` gives `hi`, the `fcom` polarity) as written, so `manual_clamp` is allowed. See
// `ffx_glow_lut_s8` and `ffx_glow_lut_u8`.
#![allow(clippy::manual_clamp)]

/// The fast float to byte pack: `round_to_f32(value * scale + 512.0)`, read back through the +512
/// (2^9) offset. The exe runs `fmul [scale]; fadd [512.0]; fstp dword; mov; shr 14`. The `fstp dword`
/// rounds the `f64` sum to `f32`; adding 512.0 puts the integer at mantissa bit 14, so
/// `(f32_bits >> 14) & 0xff` is the round-to-nearest-even byte. `value` is the operand widened to `f64`
/// (an `fld dword` `f32` or an `fild dword` `i32`, both exact) and `scale` an `fmul dword` operand,
/// 255.0 or 2.55 (the `f32` 2.549999952316284).
#[must_use]
pub fn ffx_pack_byte(value: f64, scale: f32) -> u8 {
    let f = ((value * f64::from(scale)) + 512.0) as f32;
    ((f.to_bits() >> 14) & 0xff) as u8
}

/// The colour quantize in the rectangle pass render, `0x6cb020`: a scene intensity `* 255` and the
/// active player's stat percent (`min(stat, 100)`, an integer) `* 2.55`, each packed to a byte. With no
/// active player the percent byte is 0 (the exe runs `fld 0.0; fadd 512`).
#[must_use]
pub fn ffx_glow_color_quantize(scene_intensity: f32, player_pct: Option<u8>) -> (u8, u8) {
    let b_scene = ffx_pack_byte(f64::from(scene_intensity), 255.0);
    let b_pct = match player_pct {
        Some(p) => ffx_pack_byte(f64::from(i32::from(p.min(100))), 2.55_f32),
        None => 0, // fld 0.0; fadd 512.0 gives (0x44000000 >> 14) & 0xff = 0
    };
    (b_scene, b_pct)
}

/// The death tint pack in the death pass render, `0x6cb930`: a scene intensity `* 255` packed to a
/// byte and composed with the colour `0x005393a8` as `(byte << 24) | 0x5393a8`.
#[must_use]
pub fn ffx_death_pass_pack(scene_intensity: f32) -> u32 {
    let b = ffx_pack_byte(f64::from(scene_intensity), 255.0);
    (u32::from(b) << 24) | 0x0053_93a8
}

/// The four texel-centre UV scalars from [`ffx_fullscreen_quad_uv`]: the far-corner UV `(u, v)` and
/// the half-texel offsets `(half_u, half_v)`. The exe scatters them across its eight-float UV output
/// array; that is plain stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FfxQuadUv {
    pub u: f32,
    pub v: f32,
    pub half_u: f32,
    pub half_v: f32,
}

/// The full-screen quad's UV half, in `0x6cd580`: the texel-centre UV `uv = (src_dim + 0.5) * (1 / dst_dim)`
/// and the half texel `0.5 * (1 / dst_dim)`, for a blit from `src` to `dst`. The x87 stack keeps `1 / dst_w`
/// in a register (`f64`) but spills `1 / dst_h` through an `fstp dword` (rounded to `f32`) before reuse,
/// so `u` and `half_u` multiply the `f64` `1 / dst_w` and `v` and `half_v` the `f32`-rounded
/// `1 / dst_h`.
#[must_use]
pub fn ffx_fullscreen_quad_uv(src_w: i32, src_h: i32, dst_w: i32, dst_h: i32) -> FfxQuadUv {
    let a_w = f64::from(src_w) + 0.5; // fild [esi]   ; fadd 0.5  (stays f64)
    let a_h = f64::from(src_h) + 0.5; // fild [esi+4] ; fadd 0.5  (stays f64)
    let inv_w = 1.0_f64 / f64::from(dst_w); // fild [eax]   ; fdivr 1.0 (stays f64 in st0)
    let inv_h_f32 = (1.0_f64 / f64::from(dst_h)) as f32; // fild [eax+4]; fdivr 1.0; fstp dword (->f32)
    let inv_h = f64::from(inv_h_f32);
    FfxQuadUv {
        u: (inv_w * a_w) as f32,          // fmul st,st(2)
        v: (inv_h * a_h) as f32,          // fld inv_h(f32) ; fmul st,st(1)
        half_u: (0.5_f64 * inv_w) as f32, // fld 0.5 ; fmul st,st(1)=inv_w
        half_v: (inv_h * 0.5_f64) as f32, // fld inv_h(f32) ; fmul 0.5
    }
}

/// The full-screen quad's corner half, in `0x6cd580`, on the branch taken when `[gxDevice+0x1fc]` is 0:
/// the two half-pixel-inset far corners from the integer NDC dimensions,
/// `(x_far = dim_b + 0.5, y_far = dim_a - 0.5)`. Each dimension is `fild`ed, spilled by `fstp dword`
/// (rounded to `f32`), reloaded, then offset by 0.5.
#[must_use]
pub fn ffx_fullscreen_quad_corner(dim_a: i32, dim_b: i32) -> (f32, f32) {
    let fa = f64::from(dim_a as f32); // fild ; fstp dword (->f32) ; fld
    let fb = f64::from(dim_b as f32);
    ((fb + 0.5_f64) as f32, (fa - 0.5_f64) as f32)
}

/// The affine UV map per quad corner in `0x6cd750`: `v' = uv * scale + offset` per axis. The exe runs
/// `fld dword [uv]; fmul dword [scale]; fadd dword [offset]; fstp dword [out]` at 53-bit precision, so
/// `uv * scale` stays in a register and rounds to `f32` once, at the final store; multiply and add
/// compose in `f64`. Rounding the product to `f32` first gives a different result for some corners. The vertex
/// buffer allocation, fill and submit around the loop belong to the graphics layer.
#[must_use]
pub fn ffx_transform_quad_verts(uv: [f32; 2], scale: [f32; 2], offset: [f32; 2]) -> [f32; 2] {
    [
        ((f64::from(uv[0]) * f64::from(scale[0])) + f64::from(offset[0])) as f32,
        ((f64::from(uv[1]) * f64::from(scale[1])) + f64::from(offset[1])) as f32,
    ]
}

/// The FFX blur-pyramid render-target dims from [`ffx_compute_rt_dims`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FfxRtDims {
    /// The clamped full render target `{w, h}` (globals `0xce8b5c`, `0xce8b78`).
    pub full: (i32, i32),
    /// The half-size `{w, h}` (`0xce8b20`, `0xce8b3c`).
    pub half: (i32, i32),
    /// The quarter-size `{w, h}` (`0xce8ae8`, `0xce8b04`).
    pub quarter: (i32, i32),
    /// `1.0 / raw_w` (`0xce8bac`).
    pub inv_w: f32,
    /// `1.0 / raw_h` (`0xce8bb0`).
    pub inv_h: f32,
}

/// Signed round-toward-zero `x / 2`: the exe's `cdq; sub eax,edx; sar eax`.
#[inline]
fn ffx_sar_half(x: i32) -> i32 {
    x.wrapping_sub(x >> 31) >> 1
}
/// Signed round-toward-zero `x / 4`: the exe's `cdq; and edx,3; add eax,edx; sar eax,2`.
#[inline]
fn ffx_sar_quarter(x: i32) -> i32 {
    x.wrapping_add((x >> 31) & 3) >> 2
}
/// `min(x, cap)` (signed `cmp`, `jl`) then a floor at 8 (unsigned `cmp`, `ja`), at `0x6cdbe8` to `0x6cdc08`.
#[inline]
fn ffx_clamp_dim(x: i32, cap: i32) -> i32 {
    let m = if x < cap { x } else { cap };
    if (m as u32) > 8 {
        m
    } else {
        8
    }
}

/// The render-target dimension chain of `0x6cdb40`, which sizes the glow and blur pyramid. The window
/// `w, h` come from `GxCapsWindowSize`, already truncated by `__ftol`; `cap` is the maximum texture
/// size (`caps[0x60 + 4 * idx]`). The function computes the per-axis reciprocals `1.0 / w` and
/// `1.0 / h` (`fild; fdivr [1.0 @0x7ff9d8]; fstp dword`) from the raw, unclamped `w, h`; the integer
/// half and quarter of the clamped full dimensions (signed, toward zero); and the clamp `min(x, cap)`
/// then the unsigned floor at 8. The render-target allocation at `0x6cdcb0` reads these dimensions by
/// value.
#[must_use]
pub fn ffx_compute_rt_dims(win_w: i32, win_h: i32, cap: i32) -> FfxRtDims {
    let inv_w = (1.0_f64 / f64::from(win_w)) as f32; // reciprocals use the RAW dims
    let inv_h = (1.0_f64 / f64::from(win_h)) as f32;
    let full_w = ffx_clamp_dim(win_w, cap);
    let full_h = ffx_clamp_dim(win_h, cap);
    FfxRtDims {
        full: (full_w, full_h),
        half: (ffx_sar_half(full_w), ffx_sar_half(full_h)),
        quarter: (ffx_sar_quarter(full_w), ffx_sar_quarter(full_h)),
        inv_w,
        inv_h,
    }
}

/// The frame phase in the glow pass render, `0x6cb310`: the `OsGetAsyncTimeMs` tick is reduced by an
/// integer `div` to a remainder in `[0, divisor)`, then `fild qword; fidiv [divisor]; fstp dword`
/// gives a phase in `[0, 1)`. `divisor` is 3174 (`0xce89c4`) or 2805 (`0xce89c8`).
#[must_use]
pub fn ffx_glow_phase(time_ms: u32, divisor: u32) -> f32 {
    (f64::from(time_ms % divisor) / f64::from(divisor)) as f32
}

/// The UV scroll in the glow pass render, `0x6cb310`: `counter * k * (1 / 128)` (`fild dword; fmul [k];
/// fmul [1/128 @0x80a1f0]; fstp dword`). `k` is 0.88 (`0xce89c0`, the U axis) or 1.0 (`0xce89bc`, the V axis).
#[must_use]
pub fn ffx_glow_uv_scroll(counter: i32, k: f32) -> f32 {
    const INV_128: f64 = 1.0 / 128.0; // 0x80a1f0 = 0x3c000000, power-of-two-exact
    ((f64::from(counter) * f64::from(k)) * INV_128) as f32
}

/// The desaturation scale in the glow pass render, `0x6cb310`: `3.0 * value * 0.25`
/// (`fld [3.0 @0x81147c]; fmul value; fmul [0.25 @0x8029b0]; fstp dword`). The death-active branch
/// folds `value = 1.0` to 0.75; the other branch scales a runtime tint member.
#[must_use]
pub fn ffx_glow_death_fade(value: f32) -> f32 {
    ((3.0_f64 * f64::from(value)) * 0.25_f64) as f32
}

/// One signed-byte entry of the 0x80 by 0x80 glow-wave table generator `0x6cbea0`, on the path taken
/// when `[0xce8994]` is 8. The exe runs `fild i; fmul [1/128]; fmul [2π]; fsin; fmul [128 @0x80653c];`
/// the clamp, then `__ftol`. This function is everything except the `fsin`: `v = sine * 128`, the
/// clamp (`v < -128` gives -128, `v >= 127` gives 127) and the truncation toward zero to a byte. The
/// `fsin` is the x87 transcendental, which differs between CPUs; `sine` is its result, in `[-1, 1]`,
/// and the pack is exact for any value of it.
#[must_use]
pub fn ffx_glow_lut_s8(sine: f64) -> u8 {
    let v = sine * 128.0; // fmul [128]: stays an f64 in the register the clamp and ftol read
    let c = if v < -128.0 {
        -128.0
    } else if v >= 127.0 {
        127.0
    } else {
        v
    };
    (benilla_re_cmath::x87::ftol(c) & 0xff) as u8 // __ftol = truncate toward zero, stored as a byte
}

/// One unsigned-byte entry of the glow-wave table generator `0x6cbea0`, on the path taken when
/// `[0xce8994]` is not 8. The exe runs `... fsin; fmul [0.5 @0x7ffa24]; fadd [0.5]; fmul [255 @0x7ffe58];`
/// the clamp, then `__ftol`. Without the `fsin`: `v = (sine * 0.5 + 0.5) * 255`, the clamp (`v < 0`
/// gives 0, `v >= 255` gives 255) and the truncation to a byte. See [`ffx_glow_lut_s8`] on the
/// `fsin`.
#[must_use]
pub fn ffx_glow_lut_u8(sine: f64) -> u8 {
    let v = ((sine * 0.5) + 0.5) * 255.0; // fmul [0.5]; fadd [0.5]; fmul [255]
    let c = if v < 0.0 {
        0.0
    } else if v >= 255.0 {
        255.0
    } else {
        v
    };
    (benilla_re_cmath::x87::ftol(c) & 0xff) as u8
}
