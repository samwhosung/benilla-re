//! The OpenGL device's matrix, colour, viewport and rect arithmetic (`Texture.cpp`, `TextureCache.cpp` and
//! the `CGxDevice` files, `0x58ca60` to `0x5a0000`). Each function is a block or the whole of a device
//! function, named by its address.

/// The Z column flip, `0x5a0000` (`__stdcall (src, dst)`, `ret 8`).
///
/// Copies a row-major 4 by 4 `f32` matrix with the Z column (elements 2, 6, 10 and 14) negated: each
/// element goes through the x87 and the four Z entries through `fchs`, then `rep movs` writes the
/// result. `fchs` flips the sign bit and the `fld`, `fstp` round trip is exact for any non-NaN `f32`, so
/// this is a copy with those four signs flipped. The callers are `0x59d120` (`0x59d2de`) and
/// `0x5a0090` (`0x5a0112`).
#[must_use]
pub fn z_flip(m: &[f32; 16]) -> [f32; 16] {
    let mut d = *m;
    d[2] = -m[2];
    d[6] = -m[6];
    d[10] = -m[10];
    d[14] = -m[14];
    d
}

/// A byte colour channel to a float, a slice of `0x59c100` (`0x59c13d` to `0x59c14d`).
///
/// `f = (int)b * (1/255)`: `fild`, then `fmul` by the `f32` `1/255` (`0x3b808081` at `0x809ffc`),
/// stored as `f32`. The `fmul` runs at 53-bit precision, so it is an `f64` multiply rounded once to
/// `f32`; `b` widens to `f64` exactly.
#[must_use]
pub fn color_norm(b: u8) -> f32 {
    const INV_255: f32 = f32::from_bits(0x3b80_8081); // 1/255f, the exe's [0x809ffc]
    (f64::from(i32::from(b)) * f64::from(INV_255)) as f32
}

/// The scalar-modulated colour unpack, a slice of `0x593040` (`0x593135` to `0x59317d`, inlined three
/// times in the function).
///
/// The four bytes of a packed `u32` colour each become a float scaled by `scalar * (1/255)`, the `f32`
/// `1/255` at `0x8026c8`. The scale `s = scalar * (1/255)` is computed once at 53-bit precision and
/// kept as an `f64`, not rounded back to `f32`; each byte is then `(f64)byte * s`, stored as `f32`.
/// This differs from `color_norm(byte) * scalar`, where `byte / 255` rounds to `f32` first. The output
/// order is the exe's `[b2, b1, b0, b3]`, the blue and red bytes swapped.
#[must_use]
pub fn color_modulate(packed: u32, scalar: f32) -> [f32; 4] {
    const INV_255: f32 = f32::from_bits(0x3b80_8081); // 1/255f, the exe's [0x8026c8]
    let s = f64::from(scalar) * f64::from(INV_255); // scalar·1/255 at 53-bit (f64), not re-rounded to f32
    let b = packed.to_le_bytes(); // [byte0, byte1, byte2, byte3]
    [
        (f64::from(b[2]) * s) as f32, // -> [ebp-0x18]
        (f64::from(b[1]) * s) as f32, // -> [ebp-0x14]
        (f64::from(b[0]) * s) as f32, // -> [ebp-0x10]
        (f64::from(b[3]) * s) as f32, // -> [ebp-0xc]
    ]
}

/// The viewport rect in pixels, a slice of `GxApplyViewportGL` (`0x59c730`, `0x59c786` to `0x59c7cc`,
/// before the `glViewport`, `glDepthRange` and `glScissor` calls; `systems/ui/modelview.md#clip-and-depth-clear`).
///
/// Two parameters drive the two axes, `px` (`[ebp-0x10]`) and `py` (`[ebp-0x14]`), against four device
/// scale constants (`[esi+0xf38]` to `[esi+0xf44]`). Each edge is scaled at 53-bit precision (an `f64`
/// multiply) and truncated toward zero by `__ftol`; the extent is the integer difference of the two
/// truncated edges:
///
/// ```text
/// x = trunc(px*sx0)   width  = trunc(px*sx1) - trunc(px*sx0)
/// y = trunc(py*sy0)   height = trunc(py*sy1) - trunc(py*sy0)
/// ```
///
/// Each edge rounds on its own before the subtraction, so `width` is not `trunc(px*(sx1 - sx0))`. The
/// domain is products within `i32`, the pixel range; the subtraction is the exe's 32-bit `sub`, hence
/// `wrapping_sub`.
#[must_use]
pub fn viewport(px: f32, py: f32, sx0: f32, sx1: f32, sy0: f32, sy1: f32) -> [i32; 4] {
    let x0 = (f64::from(px) * f64::from(sx0)) as i32; // trunc(px·[esi+0xf38])
    let x1 = (f64::from(px) * f64::from(sx1)) as i32; // trunc(px·[esi+0xf3c])
    let y0 = (f64::from(py) * f64::from(sy0)) as i32; // trunc(py·[esi+0xf40])
    let y1 = (f64::from(py) * f64::from(sy1)) as i32; // trunc(py·[esi+0xf44])
    [x0, y0, x1.wrapping_sub(x0), y1.wrapping_sub(y0)] // glViewport(edi, ebx, ecx, eax)
}

/// The aspect fit of one rect edge, `0x5910a0` (the head of a family of four with `0x5910e0`,
/// `0x591120` and `0x591160`; `__thiscall`, `this` the rect, no stack arguments).
///
/// The family re-fits one edge of an integer rect `r = [f0, f4, f8, fC]` (the X pair `f0` and `f8`, the Y
/// pair `f4` and `fC`) to the device scale `f32` at `[0xc0ef60]`. Each first tests the scale against
/// `0.0` (`fld [0xc0ef60]; fcomp 0.0; jnp`) and leaves the rect alone when it is zero; it then derives
/// the edge from the other axis' extent at 53-bit precision (an `f64` divide or multiply) and truncates
/// with `__ftol` (`0x40a2b0`):
///
/// ```text
/// 0x5910a0  min_x:  f0 = f8 - trunc((f64)(fC - f4) / scale)
/// 0x5910e0  max_x:  f8 = f0 + trunc((f64)(fC - f4) / scale)
/// 0x591120  min_y:  f4 = fC - trunc((f64)(f8 - f0) * scale)
/// 0x591160  max_y:  fC = f4 + trunc((f64)(f8 - f0) * scale)
/// ```
///
/// The extent subtraction and the final add or subtract are 32-bit integer operations (`wrapping_*`);
/// the extent widens through `fild`, exact for `i32`, before the `f32` scale divides or multiplies it.
#[must_use]
pub fn aspect_fit_min_x(r: [i32; 4], scale: f32) -> [i32; 4] {
    let mut o = r;
    if scale != 0.0 {
        let ext = r[3].wrapping_sub(r[1]); // fC − f4 (32-bit sub)
        let q = (f64::from(ext) / f64::from(scale)) as i32; // fild; fdiv [0xc0ef60] @53-bit; __ftol trunc
        o[0] = r[2].wrapping_sub(q); // f8 − q
    }
    o
}

/// The aspect fit, max-X edge, `0x5910e0`: `f8 = f0 + trunc((f64)(fC - f4) / scale)`, guarded on
/// `scale != 0` (see [`aspect_fit_min_x`]).
#[must_use]
pub fn aspect_fit_max_x(r: [i32; 4], scale: f32) -> [i32; 4] {
    let mut o = r;
    if scale != 0.0 {
        let ext = r[3].wrapping_sub(r[1]); // fC − f4
        let q = (f64::from(ext) / f64::from(scale)) as i32; // fild; fdiv [0xc0ef60] @53-bit; __ftol trunc
        o[2] = r[0].wrapping_add(q); // f0 + q
    }
    o
}

/// The aspect fit, min-Y edge, `0x591120`: `f4 = fC - trunc((f64)(f8 - f0) * scale)`, guarded on
/// `scale != 0` (see [`aspect_fit_min_x`]).
#[must_use]
pub fn aspect_fit_min_y(r: [i32; 4], scale: f32) -> [i32; 4] {
    let mut o = r;
    if scale != 0.0 {
        let ext = r[2].wrapping_sub(r[0]); // f8 − f0
        let q = (f64::from(ext) * f64::from(scale)) as i32; // fild; fmul [0xc0ef60] @53-bit; __ftol trunc
        o[1] = r[3].wrapping_sub(q); // fC − q
    }
    o
}

/// The aspect fit, max-Y edge, `0x591160`: `fC = f4 + trunc((f64)(f8 - f0) * scale)`, guarded on
/// `scale != 0` (see [`aspect_fit_min_x`]).
#[must_use]
pub fn aspect_fit_max_y(r: [i32; 4], scale: f32) -> [i32; 4] {
    let mut o = r;
    if scale != 0.0 {
        let ext = r[2].wrapping_sub(r[0]); // f8 − f0
        let q = (f64::from(ext) * f64::from(scale)) as i32; // fild; fmul [0xc0ef60] @53-bit; __ftol trunc
        o[3] = r[1].wrapping_add(q); // f4 + q
    }
    o
}

/// The display gamma ramp builder, `0x591680` (`__thiscall (table, gamma)`, `ret 4`).
///
/// Fills a Windows `GAMMARAMP` (3 by 256 `u16`: red, green, blue) from one gamma exponent. For each
/// `i` in `0..256` it computes `entry = __ftol(pow(i * (1/255), gamma) * 65535)` and writes the same
/// word into all three channel tables (`[base + 2i]`, `[base + 0x200 + 2i]`, `[base + 0x400 + 2i]`).
/// The `1/255` is the `f32` at `0x8026c8` (`0x3b808081`) and `65535` the `f32` at `0x809d98`
/// (`0x477fff00`); `i` is widened by `fild`, and both `fmul`s run at 53-bit precision, so the chain is
/// `f64`, truncated to `i64` by `__ftol` and kept as the low 16 bits (`ax`).
///
/// The exponentiation is the exe's own x87 `pow` (`0x73f940`, `fyl2x` and `f2xm1`), a transcendental
/// that differs between CPUs; this uses the host's `powf`. The final truncation to an integer word
/// absorbs a sub-ulp difference between the two, and the table matches the exe's for every exponent
/// the test draws.
#[must_use]
pub fn gamma_ramp(gamma: f32) -> [u16; 768] {
    const INV_255: f32 = f32::from_bits(0x3b80_8081); // 1/255f, [0x8026c8]
    const SCALE: f32 = f32::from_bits(0x477f_ff00); // 65535.0f, [0x809d98]
    let mut ramp = [0u16; 768];
    for i in 0..256u32 {
        let base = f64::from(i as i32) * f64::from(INV_255); // fild qword; fmul 1/255 at 53-bit
        let curve = base.powf(f64::from(gamma)); // pow(base, gamma), `0x73f940` (x87 fyl2x/f2xm1)
        let scaled = curve * f64::from(SCALE); // fmul 65535 at 53-bit
        let word = (scaled as i64) as u16; // __ftol truncates toward zero; device keeps `ax`
        ramp[i as usize] = word; // Red   [base + 2i]
        ramp[256 + i as usize] = word; // Green [base + 0x200 + 2i]
        ramp[512 + i as usize] = word; // Blue  [base + 0x400 + 2i]
    }
    ramp
}

/// The viewport aspect ratio, the tail of `0x591fd0` (`0x592021` to `0x592038`).
///
/// After it copies its 14-dword parameter block and clamps the extents at `[this+0x214]` (width) and
/// `[this+0x218]` (height), the function stores `width / height` to the `f32` global `[0xc0ef60]` when
/// the flag `[this+0x20c]` is set: `fild dword [this+0x214]; fidiv dword [this+0x218]; fstp dword
/// [0xc0ef60]`. The division is done on the integers at 53-bit precision and rounded once to `f32`:
/// `((width as f64) / (height as f64)) as f32`, not a divide in `f32`.
#[must_use]
pub fn aspect_ratio(width: i32, height: i32) -> f32 {
    (f64::from(width) / f64::from(height)) as f32 // fild width; fidiv height (53-bit); fstp f32
}

/// The third matrix row negated, a block of `0x5926c0` (`0x5926f3` to `0x592723`).
///
/// The function copies a cached 4 by 4 matrix (`this+0xf90`) to the caller's buffer and, when the
/// capability query `0x589ca0` returns 0 (a vtable call), negates the third row (elements 8 to 11, byte
/// offsets `+0x20` to `+0x2c`) by multiplying each by the `f32` `-1.0` at `0x8030a0`: `fld; fmul;
/// fstp`. The multiply runs at 53-bit precision and `x * -1.0` is an exact sign flip for every finite
/// or infinite input, so each element is `(f64(x) * -1.0) as f32`.
#[must_use]
pub fn negate_z_row(row: &[f32; 4]) -> [f32; 4] {
    const NEG_ONE: f32 = f32::from_bits(0xbf80_0000); // -1.0f, [0x8030a0]
    let mut out = [0f32; 4];
    for (o, &x) in out.iter_mut().zip(row.iter()) {
        *o = (f64::from(x) * f64::from(NEG_ONE)) as f32; // fld; fmul -1.0 (53-bit); fstp f32
    }
    out
}

/// A component-wise `f32` vec3 subtract, a block of `0x593330` (`0x593387` to `0x5933b7`).
///
/// After the colour modulate of `0x593040`, when the source's flag bit `0x2` is set and the three
/// offset components are not all `>= 0.0` (`fcomp [0x7ffd74]`), the function subtracts the offset vec3
/// from the element's stored vec3 in place: `dst[k] -= v[k]` for `k` = 0, 1, 2 by `fld dst[k]; fsub
/// v[k]; fstp dst[k]` (the third reads `v[2]` from the spill `[ebp-0x8]`). Each `fsub` runs at 53-bit
/// precision on the widened operands and rounds to `f32`: `((a as f64) - (b as f64)) as f32` per lane.
#[must_use]
pub fn sub_vec3(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) - f64::from(b[0])) as f32, // fld dst[0]; fsub st(2)=v0; fstp dst[0]
        (f64::from(a[1]) - f64::from(b[1])) as f32, // fld dst[1]; fsub st(1)=v1; fstp dst[1]
        (f64::from(a[2]) - f64::from(b[2])) as f32, // fld dst[2]; fsub [ebp-8]=v2; fstp dst[2]
    ]
}

/// The light vector negate, a block of `0x59c820` (`0x59c90c` to `0x59c930`, inside its per-light
/// loop).
///
/// Before `glLightfv(GL_LIGHT0+i, GL_POSITION, v)` the function passes the four floats unchanged when
/// `w == 1.0f` (`[esi-0x40]`); otherwise it negates all four with `fchs` (`fld; fchs; fstp` per
/// element) into a stack temporary and passes that. This is the negate: a sign flip of `[x, y, z, w]`
/// at `[esi-0x4c]` to `[esi-0x40]`. `fchs` flips the sign bit, and the `fld`, `fstp` round trip is exact
/// for any non-NaN `f32` (a zero changes sign), so it is unary `-` bit for bit.
#[must_use]
pub fn light_dir_negate(v: &[f32; 4]) -> [f32; 4] {
    [-v[0], -v[1], -v[2], -v[3]] // fchs ×4 -> [ebp-0x20..-0x14], passed to glLightfv(GL_POSITION)
}

/// The projection matrix Z row negate, a slice of `0x59fe20` (`0x59fe2c` to `0x59fea0`).
///
/// The function copies the caller's row-major 4 by 4 matrix into a stack matrix, negating the third row
/// (elements 8 to 11, source byte offsets `0x20` to `0x2c`) with `fchs` and copying the other twelve
/// with `fld; fstp`. This differs from [`z_flip`], which negates the Z column (2, 6, 10 and 14). The
/// conditional depth-bias multiply after it calls the maths library's 4 by 4 multiply (`0x7bc6a0`);
/// the Y column scale is [`proj_ycol_scale`].
#[must_use]
pub fn proj_zrow_negate(m: &[f32; 16]) -> [f32; 16] {
    let mut d = *m;
    d[8] = -m[8]; // fld [eax+0x20]; fchs; fstp [ebp-0x20]
    d[9] = -m[9]; // fld [eax+0x24]; fchs; fstp [ebp-0x1c]
    d[10] = -m[10]; // fld [eax+0x28]; fchs; fstp [ebp-0x18]
    d[11] = -m[11]; // fld [eax+0x2c]; fchs; fstp [ebp-0x14]
    d
}

/// The projection matrix Y column scale by -1, a slice of `0x59fe20` (`0x59ff8e` to `0x59ffc5`).
///
/// On an off-screen target the function multiplies the second column (elements 1, 5, 9 and 13) by the
/// `f32` `-1.0` (`0xbf800000` at `0x8030a0`): `fld; fmul [0x8030a0]; fstp` per element. The multiply
/// runs at 53-bit precision and rounds to `f32`; `x * -1.0` is exact for every finite, infinite or zero
/// input, so the result equals a sign flip. It is written as the multiply the exe runs.
#[must_use]
pub fn proj_ycol_scale(x: f32) -> f32 {
    const NEG_ONE: f32 = f32::from_bits(0xbf80_0000); // -1.0f, the exe's [0x8030a0]
    (f64::from(x) * f64::from(NEG_ONE)) as f32 // fmul at 53-bit (f64), stored f32
}

/// The screen-space textured quad corners of `0x5956f0` (`0x5959a0` to `0x595b60`).
///
/// The function draws a 32-unit textured quad (four vertices, uv `(0,0)` to `(1,1)`, colour
/// `0xffffffff`) at a screen position. From two integer pixel coordinates (`c1`, `c2`), two device
/// origins (`off1 = [esi+0x2888]`, `off2 = [esi+0x288c]`) and the viewport reciprocals it computes the
/// four corners' NDC `(x, y)`. Each axis reciprocal is guarded: `r = rh > 0.0 ? 1.0/rh : 0.0` (`fld 1.0;
/// fdiv` at 53-bit precision, stored as `f32` at `[ebp-0x20]` and `[ebp-0x4]`, so it is rounded to
/// `f32` before reuse), with `rh = [rect+0xc]` and `rw = [rect+0x8]`.
///
/// The integer extents are loaded by `fild qword` with a zero high dword, so a negative `c - off`
/// reads as its unsigned 32-bit value (`c.wrapping_sub(off) as u32`); they are scaled at 53-bit
/// precision and kept on the x87 stack, rounded to `f32` only when stored into a vertex:
///
/// ```text
/// V0 = u32(c1-off1)        * (1/rh)      W2 = 1.0 - u32(c2-off2)        * (1/rw)
/// V1 = u32(c1-off1 + 0x20) * (1/rh)      W3 = 1.0 - u32(c2-off2 + 0x20) * (1/rw)
/// ```
///
/// When `[esi+0x1fc] == 1`, a half-texel centring shifts each in `f64` before the store: `V -= (1/rh) *
/// 0.5` and `W += (1/rw) * 0.5`. The corners are `(V0,W2)`, `(V0,W3)`, `(V1,W2)`, `(V1,W3)`, vertices
/// 0 to 3 at vertex stride `0x18`. The rest of the function moves data.
#[must_use]
pub fn quad_corners(
    c1: i32,
    c2: i32,
    off1: i32,
    off2: i32,
    rh: f32,
    rw: f32,
    centered: bool,
) -> [[f32; 2]; 4] {
    // fld 1.0; fcomp 0.0 ([0x7ffd74]); guard then fdiv at 53-bit, stored f32 ([ebp-0x20]/[ebp-0x4]).
    let ry: f32 = if rh > 0.0 {
        (1.0_f64 / f64::from(rh)) as f32
    } else {
        0.0
    };
    let rx: f32 = if rw > 0.0 {
        (1.0_f64 / f64::from(rw)) as f32
    } else {
        0.0
    };
    // fild qword (hi dword = 0) → unsigned-32 of the 32-bit `c − off`; fmul r (f32→f64) at 53-bit.
    let a0 = c1.wrapping_sub(off1) as u32;
    let a1 = c1.wrapping_sub(off1).wrapping_add(0x20) as u32;
    let b0 = c2.wrapping_sub(off2) as u32;
    let b1 = c2.wrapping_sub(off2).wrapping_add(0x20) as u32;
    let mut v0 = f64::from(a0) * f64::from(ry);
    let mut v1 = f64::from(a1) * f64::from(ry);
    let mut w2 = 1.0 - f64::from(b0) * f64::from(rx); // fmul rx; fsubr 1.0
    let mut w3 = 1.0 - f64::from(b1) * f64::from(rx);
    if centered {
        let hy = f64::from(ry) * 0.5; // fmul 0.5 ([0x7ffa24])
        let hx = f64::from(rx) * 0.5;
        v0 -= hy;
        v1 -= hy;
        w2 += hx;
        w3 += hx;
    }
    let (v0, v1, w2, w3) = (v0 as f32, v1 as f32, w2 as f32, w3 as f32); // fstp dword per vertex
    [[v0, w2], [v0, w3], [v1, w2], [v1, w3]]
}

/// The render-state scale `x * -65536`, a slice of `0x59baa0` (`0x59bef7` to `0x59bf0a`).
///
/// One case of the function's render-state dispatcher converts a float parameter for the call at
/// `[0x7ff458]`: `fld [ebp+0xc]; fmul [0x80a000]; fmul [0x8030a0]; fstp [esp]`, a multiply by the `f32`
/// `65536.0` (`0x47800000`) then by `-1.0` (`0xbf800000`), the result passed with the constant `-4.0`.
/// Both multiplies run at 53-bit precision and the `fstp` rounds to `f32`. Each is exact (an exponent
/// shift and a sign flip), so the chain is exact for finite inputs; it is written as the two multiplies.
#[must_use]
pub fn scale_neg_65536(x: f32) -> f32 {
    const C65536: f32 = f32::from_bits(0x4780_0000); // 65536.0f, [0x80a000]
    const NEG_ONE: f32 = f32::from_bits(0xbf80_0000); // -1.0f, [0x8030a0]
    ((f64::from(x) * f64::from(C65536)) * f64::from(NEG_ONE)) as f32 // fmul ×2 at 53-bit; fstp f32
}

/// The matrix reorient, a slice of `0x59d120` (`0x59d2e3` to `0x59d32b`, after the `z_flip` copy of
/// `0x5a0000`).
///
/// In its texture-matrix case the function swaps the three off-diagonal pairs of the upper-left 3 by 3,
/// elements `(1,4)`, `(2,8)` and `(6,9)` (byte offsets `0x4` and `0x10`, `0x8` and `0x20`, `0x18` and
/// `0x24`), with integer `mov`s (exact, NaN preserving), then negates the translation row (elements 12,
/// 13 and 14 at `0x30`, `0x34`, `0x38`) by `fld; fmul [0x8030a0]; fstp`. The multiply runs at 53-bit
/// precision and rounds to `f32`; `x * -1.0` is exact, and it is written as the multiply the exe runs.
#[must_use]
pub fn matrix_reorient(m: &[f32; 16]) -> [f32; 16] {
    const NEG_ONE: f32 = f32::from_bits(0xbf80_0000); // -1.0f, [0x8030a0]
    let mut d = *m;
    d.swap(1, 4); // [edi+0x4]  <-> [edi+0x10]
    d.swap(2, 8); // [edi+0x8]  <-> [edi+0x20]
    d.swap(6, 9); // [edi+0x18] <-> [edi+0x24]
    d[12] = (f64::from(m[12]) * f64::from(NEG_ONE)) as f32; // [edi+0x30] · -1.0
    d[13] = (f64::from(m[13]) * f64::from(NEG_ONE)) as f32; // [edi+0x34]
    d[14] = (f64::from(m[14]) * f64::from(NEG_ONE)) as f32; // [edi+0x38]
    d
}

/// The rect shrink test, a block of `0x58ca60` (`0x58cabf` to `0x58caef`, case 0 of its message
/// dispatch).
///
/// Two rects laid out `[x0, y0, x1, y1]` (offsets `0`, `4`, `8`, `0xc`): `a` is the message argument
/// (`[ebp+0xc]`) and `b` the cached rect (the return of `0x592330`). The function computes each
/// rect's height (`y1 - y0`) and width (`x1 - x0`) with `fsub` and compares `b`'s extents with `a`'s by
/// `fcompp`. It sets a flag when `b` is strictly smaller than `a` in height or width:
/// `(b.y1 - b.y0) < (a.y1 - a.y0) || (b.x1 - b.x0) < (a.x1 - a.x0)`. The `fsub` results go straight to
/// `fcompp` and are never stored. Both operands are `f32`, so their difference is exact in `f64`, and `<`
/// matches the strict less-than of `fcompp`; a NaN is unordered and not less in both. The height
/// compare short-circuits as the first `jnp` skips the width compare.
#[must_use]
pub fn rect_b_smaller(a: &[f32; 4], b: &[f32; 4]) -> bool {
    let a_h = f64::from(a[3]) - f64::from(a[1]); // fld [edx+0xc]; fsub [edx+0x4]
    let b_h = f64::from(b[3]) - f64::from(b[1]); // fld [ecx+0xc]; fsub [ecx+0x4]
    if b_h < a_h {
        return true; // fcompp; test ah,5; jnp -> bl=1 (b shorter than a)
    }
    let a_w = f64::from(a[2]) - f64::from(a[0]); // fld [edx+0x8]; fsub [edx]
    let b_w = f64::from(b[2]) - f64::from(b[0]); // fld [ecx+0x8]; fsub [ecx]
    b_w < a_w // second fcompp; bl=1 iff b narrower than a, else xor bl,bl -> 0
}
