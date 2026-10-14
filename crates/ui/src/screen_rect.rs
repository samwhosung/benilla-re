//! Rectangles and points on the screen: the world to screen projection of the camera object
//! (`0x483ee0`) and the two rectangle clamps `0x509bf0` and `0x509e20`.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`. The screen extents come from `0x41ae60` (the x extent, `[0x832a44] * arg`) and
//! `0x41ae70` (the y extent, `[0x832a48] * arg`); the rectangle clamps call each with 1.0, so the
//! value is the bare extent, and they are inputs here.

/// What [`project_to_screen`] returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScreenProj {
    /// `znear > clip.z`: the function returns false and leaves the output untouched.
    Culled,
    /// The two `f32` written through the out pointer, and `al`: whether the point is on screen.
    Mapped { x: f32, y: f32, on_screen: bool },
}

/// `0x483ee0` (a `__thiscall` on the camera object, with a world position and an out pointer,
/// `ret 8`): the screen position of a world point. The point has the camera position subtracted and
/// is transformed by the 4 by 4 matrix at `this+0x3b0` (`0x7bcb40`), which gives `clip = [x, y, z, w]`;
/// everything after is computed here.
///
/// 1. Near cull: `znear > clip.z` (`znear` is the camera's `+0x38`, reached through `this+0x65b8`)
///    is [`ScreenProj::Culled`]. A NaN `clip.z` is not culled.
/// 2. `inv_w = 1.0 / clip.w`. The two axes round differently: `ndc_x = ((clip.x * inv_w + 1.0) * 0.5)
///    as f32`, with the product and the sum at register precision and only the last store
///    (`fstp [ebp-0x24]`) rounding; `ndc_y = ((f32(clip.y * inv_w) + 1.0) * 0.5) as f32`, where the
///    product is stored first (`fstp [ebp-0x20]`).
/// 3. The viewport map and the scale (`0x41ad80`): `screen_x = f32(scale_x * f32((vp_3ac - vp_3a4) *
///    ndc_x))` and `screen_y = f32(scale_y * f32((vp_3a8 - vp_3a0) * ndc_y))`.
/// 4. The frame origin shift: when `rect_68 < 0`, `screen_x -= rect_68`, kept at register
///    precision and rounded only by the out store; when `rect_64 < 0`, `screen_y = f32(screen_y -
///    rect_64)`. The out store and the on-screen test both see that rounded y, but the register
///    value of x.
/// 5. `out[0] = f32(screen_x)`, `out[1] = screen_y`, and `on_screen` holds when `0 <= screen_x <=
///    rect_70 - rect_68` and `0 <= screen_y <= rect_6c - rect_64` (below 0 fails; above the upper
///    edge fails; equal passes).
///
/// `vp_3a0` to `vp_3ac` are the viewport fields at `this+0x3a0` to `+0x3ac`, `scale_x` and `scale_y`
/// the globals `0x832a44` and `0x832a48`, and `rect_64` to `rect_70` the fields at `this+0x64` to
/// `+0x70`.
#[allow(clippy::too_many_arguments)]
pub fn project_to_screen(
    clip: [f32; 4],
    znear: f32,
    vp_3a0: f32,
    vp_3a4: f32,
    vp_3a8: f32,
    vp_3ac: f32,
    scale_x: f32,
    scale_y: f32,
    rect_64: f32,
    rect_68: f32,
    rect_6c: f32,
    rect_70: f32,
) -> ScreenProj {
    if f64::from(znear) > f64::from(clip[2]) {
        return ScreenProj::Culled;
    }

    let invw = 1.0 / f64::from(clip[3]);
    let ndc_x = ((f64::from(clip[0]) * invw + 1.0) * 0.5) as f32;
    let clipy_inv = (f64::from(clip[1]) * invw) as f32;
    let ndc_y = ((f64::from(clipy_inv) + 1.0) * 0.5) as f32;

    let span_y = f64::from(vp_3a8) - f64::from(vp_3a0);
    let span_x = f64::from(vp_3ac) - f64::from(vp_3a4);
    let bx = (span_x * f64::from(ndc_x)) as f32;
    let ay = (span_y * f64::from(ndc_y)) as f32;
    let screen_x0 = (f64::from(scale_x) * f64::from(bx)) as f32;
    let screen_y0 = (f64::from(scale_y) * f64::from(ay)) as f32;

    let mut sx = f64::from(screen_x0);
    if rect_68 < 0.0 {
        sx -= f64::from(rect_68);
    }
    let out_x = sx as f32;
    let sy_f32 = if rect_64 < 0.0 {
        (f64::from(screen_y0) - f64::from(rect_64)) as f32
    } else {
        screen_y0
    };
    let sy = f64::from(sy_f32);

    let width_x = f64::from(rect_70) - f64::from(rect_68);
    let height_y = f64::from(rect_6c) - f64::from(rect_64);
    let on_screen = sx >= 0.0 && sx <= width_x && sy >= 0.0 && sy <= height_y;

    ScreenProj::Mapped {
        x: out_x,
        y: sy_f32,
        on_screen,
    }
}

/// 0.0375, the `f32` at `[0x8087cc]`: subtracted from the y extent for the high edge.
const C_8087CC: f32 = f32::from_bits(0x3d19_999a);
/// 0.01875, the `f32` at `[0x8087d0]`: the low y edge.
const C_8087D0: f32 = f32::from_bits(0x3c99_999a);
/// 0.0, `[0x8087d4]`: the low x edge.
const C_8087D4: f32 = 0.0;
/// 0.0, `[0x8087d8]`: subtracted from the x extent for the high edge.
const C_8087D8: f64 = 0.0;

/// `0x509bf0` (`systems/ui/smartrect.md#pushes-by-region`): the outcode of a rectangle against the
/// screen, and its move onto the screen that keeps its size. `rect = [top, left, bottom, right]`
/// (the four stack floats at `[ebp+8..+0x14]`). The edges, after the two extent reads (`0x509c01`,
/// `0x509c0e`): `x_lo = 0`, `x_hi = (W - 0.0) as f32` (`0x509c23`), `y_lo = 0.01875` and
/// `y_hi = (H - 0.0375) as f32` (`0x509c2c`).
///
/// The sizes are stored as `f32` (`0x509c69`, `0x509c72`): `height = (top - bottom) as f32` and
/// `width = (right - left) as f32`. The outcode, with plain ordered compares so a NaN sets no bit, is
///
/// ```text
/// bit 0 (0x1): left   < x_lo     (0x509c75)
/// bit 1 (0x2): right  > x_hi     (0x509c8a)
/// bit 2 (0x4): top    > y_hi     (0x509cad)
/// bit 3 (0x8): bottom < y_lo     (0x509c9a)
/// ```
///
/// `mode` is the argument kept in `edi` (`test edi, edi` at `0x509cbd`). With a nonzero `mode` the
/// function returns the outcode and does not write the rectangle (`None`). With `mode` 0 and outcode
/// 0 the rectangle is copied as it is (`0x509cc9`). Otherwise each violated edge is moved to the
/// screen's and its partner follows at the original size: the vertical pair first (`0x509cea`, bit 2
/// before bit 3), then the horizontal pair (`0x509d1e`, bit 0 before bit 1). The edge is an exact
/// copy and the partner an `f32` store:
///
/// ```text
/// bit 2: top = y_hi;     bottom = (y_hi - height) as f32   (0x509cf4)
/// bit 3: bottom = y_lo;  top = (y_lo + height) as f32      (0x509d0a)
/// bit 0: left = x_lo;    right = (x_lo + width) as f32     (0x509d28)
/// bit 1: right = x_hi;   left = (x_hi - width) as f32      (0x509d44)
/// ```
pub fn outcode_and_clamp_rect(
    rect: [f32; 4],
    mode: u32,
    extent_x: f32,
    extent_y: f32,
) -> (u32, Option<[f32; 4]>) {
    let [top, left, bottom, right] = rect;

    let x_lo = C_8087D4;
    let y_lo = C_8087D0;
    let x_hi = (f64::from(extent_x) - C_8087D8) as f32;
    let y_hi = (f64::from(extent_y) - f64::from(C_8087CC)) as f32;

    let height = (f64::from(top) - f64::from(bottom)) as f32;
    let width = (f64::from(right) - f64::from(left)) as f32;

    let mut outcode = 0u32;
    if left < x_lo {
        outcode |= 0x1;
    }
    if right > x_hi {
        outcode |= 0x2;
    }
    if top > y_hi {
        outcode |= 0x4;
    }
    if bottom < y_lo {
        outcode |= 0x8;
    }

    if mode != 0 {
        return (outcode, None);
    }
    if outcode == 0 {
        return (outcode, Some([top, left, bottom, right]));
    }

    let mut o = [top, left, bottom, right];
    if outcode & 0xc != 0 {
        if outcode & 0x4 != 0 {
            o[0] = y_hi;
            o[2] = (f64::from(y_hi) - f64::from(height)) as f32;
        } else if outcode & 0x8 != 0 {
            o[2] = y_lo;
            o[0] = (f64::from(y_lo) + f64::from(height)) as f32;
        }
    }
    if outcode & 0x3 != 0 {
        if outcode & 0x1 != 0 {
            o[1] = x_lo;
            o[3] = (f64::from(x_lo) + f64::from(width)) as f32;
        } else if outcode & 0x2 != 0 {
            o[3] = x_hi;
            o[1] = (f64::from(x_hi) - f64::from(width)) as f32;
        }
    }
    (outcode, Some(o))
}

/// `0x509e20` (`systems/ui/smartrect.md#normalising-a-rectangle`; a `__thiscall` on the rectangle,
/// `ecx`, with no stack argument): moves a rectangle `[top, left, bottom, right]` onto
/// `[0, W] x [0, H]` in place, keeping its size. The sizes are stored as `f32` (`0x509e33`,
/// `0x509e3c`): `h = (top - bottom) as f32` and `w = (right - left) as f32`. `W` (`0x41ae60`) is
/// stored as `f32` at `[ebp-4]` (`0x509e49`); `H` (`0x41ae70`) stays on the x87 stack for the first
/// compare. Four moves follow in order, each on the rectangle as the earlier ones left it, so a later
/// move can undo an earlier one:
///
/// ```text
/// 0x509e51  top:     if H <= top :     top = H;       bottom = (H - h) as f32   (a NaN does not move it)
/// 0x509e66  bottom:  if bottom < 0 :   bottom = 0;    top = h
/// 0x509e80  left:    if left < 0 :     left = 0;      right = w
/// 0x509e99  right:   if right > W :    right = W;     left = (W - w) as f32
/// ```
///
/// The compares with 0 read `[0x7ffd74]`.
pub fn clamp_rect_into_screen(rect: [f32; 4], extent_x: f32, extent_y: f32) -> [f32; 4] {
    let mut r = rect;
    let h = (f64::from(r[0]) - f64::from(r[2])) as f32;
    let w = (f64::from(r[3]) - f64::from(r[1])) as f32;
    let cap_w = extent_x;
    let cap_h = extent_y;

    if cap_h <= r[0] {
        r[0] = cap_h;
        r[2] = (f64::from(cap_h) - f64::from(h)) as f32;
    }
    if r[2] < 0.0 {
        r[2] = 0.0;
        r[0] = h;
    }
    if r[1] < 0.0 {
        r[1] = 0.0;
        r[3] = w;
    }
    if r[3] > cap_w {
        r[3] = cap_w;
        r[1] = (f64::from(cap_w) - f64::from(w)) as f32;
    }
    r
}
