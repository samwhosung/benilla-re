//! The arithmetic inside the font instance and multi-line text layout functions
//! (`0x771ec0` to `0x772d40`, `systems/ui/fontstring.md#size-and-fitting`,
//! `systems/ui/tooltipcontent.md`): scale products, divisions by the widget scale `+0x7c`,
//! change tests and a rect comparison. Each function says which slices it is. A sum the code leaves
//! live on the x87 stack is an f64; only the `fstp dword` stores narrow to `f32`.

/// `0x80c5c8`: the position-settle epsilon, about 1e-5.
const POS_EPS: f32 = f32::from_bits(0x3727_c5ac);
/// `0x8029d4`: the change epsilon, 2^-22.
const EPS: f32 = f32::from_bits(0x3480_0000);
/// `0x7ffd68`: 1024.0.
const PX1024: f32 = f32::from_bits(0x4480_0000);
/// `0x7ff9d8`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `0x832a4c`: the aspect base that `0x41ad70` loads, 1.0.
const SCALE: f32 = f32::from_bits(0x3f80_0000);
/// `0x832a44`: the X scale 0.8 that `0x41ae60` multiplies by.
const PXSCALE: f32 = f32::from_bits(0x3f4c_cccd);

/// `coord * scale` stored as an `f32` (`fld [arg]; fmul [esi+0x7c]; fstp`), the most frequent
/// product of the group: eight in `0x771ec0`, two each in `0x772180`, `0x7722a0`, `0x772b60`,
/// `0x772bc0` and `0x772c30`, five in `0x7724a0`, three in `0x7729b0`, one in `0x772890` and
/// `0x772ae0`.
pub fn scale_mul(coord: f32, scale: f32) -> f32 {
    (f64::from(coord) * f64::from(scale)) as f32
}

/// `pos * scale + off` stored as an `f32`, the product and the sum both at 53 bits (`0x772100`,
/// `0x7724a0`, `0x772d40`).
pub fn scale_offset(pos: f32, scale: f32, off: f32) -> f32 {
    ((f64::from(pos) * f64::from(scale)) + f64::from(off)) as f32
}

/// `val / scale` stored as an `f32` (`fld [val]; fdiv [esi+0x7c]`; `0x7727b0`, `0x772890`,
/// `0x7729b0`, `0x772ae0`).
pub fn div_scale(val: f32, scale: f32) -> f32 {
    (f64::from(val) / f64::from(scale)) as f32
}

/// The chain of `0x772930` and `0x772a60`: `0.8 * (1.0 / (SCALE * 1024.0))`. `fdivr` reverses the
/// divide, the quotient is narrowed to `f32` as the argument of `0x41ae60`, and the product with 0.8
/// stays an f64 for the `fcomp` that follows.
pub fn set_scaled_chain() -> f64 {
    let recip = f64::from(ONE) / (f64::from(SCALE) * f64::from(PX1024));
    f64::from(PXSCALE) * f64::from(recip as f32)
}

/// `|a - b| < 1e-5` (`0x80c5c8`) in `0x771ec0` (`0x771f88`) and `0x7724a0` (`0x7725fb`, `0x772632`):
/// `fcomp` then `test ah,5; jp`, which is not taken only for an ordered "less". False for NaN.
pub fn settled_poseps(a: f32, b: f32) -> bool {
    let d = (f64::from(a) - f64::from(b)).abs();
    !d.is_nan() && d < f64::from(POS_EPS)
}

/// `|arg - field| < 2^-22` in `0x772240` (`0x77225a`): `fsubr` is `[ebp+8] - [esi+0xf4]`, and `jnp`
/// is taken for an ordered "less", which skips the field write. False for NaN.
pub fn settled_eps(arg: f32, field: f32) -> bool {
    let d = (f64::from(arg) - f64::from(field)).abs();
    !d.is_nan() && d < f64::from(EPS)
}

/// `0x7724a0` (`0x77253a` to `0x772559`): `1.0 / scale` stays live on `st1` and multiplies both spans
/// `hi - lo` before each is stored as an `f32`.
pub fn recip_span_pair(scale: f32, hi0: f32, lo0: f32, hi1: f32, lo1: f32) -> [f32; 2] {
    let recip = f64::from(ONE) / f64::from(scale);
    [
        ((f64::from(hi0) - f64::from(lo0)) * recip) as f32,
        ((f64::from(hi1) - f64::from(lo1)) * recip) as f32,
    ]
}

/// `0x771ec0` (`0x771f5c` to `0x771f61`): the line count `{n, 0}` by `fild qword` times the live
/// coordinate on `st1`, stored as an `f32`.
pub fn linecount_scale(n: u32, coord: f32) -> f32 {
    (f64::from(n) * f64::from(coord)) as f32
}

/// `0x771ec0` (`0x771f45`): the text extent that `0x766750` returns live in `st0` plus the `f32` at
/// `[ebp-4]`. The sum stays an f64.
pub fn add_reload(live: f64, reload: f32) -> f64 {
    live + f64::from(reload)
}

/// The rect test of `0x772d40` (`0x772d62` to `0x772d80` for y, `0x772d82` to `0x772d9b` for x):
/// `|(b1 - a1) - (b2 - a2)| < 1e-5`. The `jp` that leaves the function early is taken for
/// everything else, NaN included.
pub fn rect_axis_settled(b1: f32, a1: f32, b2: f32, a2: f32) -> bool {
    let m = ((f64::from(b1) - f64::from(a1)) - (f64::from(b2) - f64::from(a2))).abs();
    !m.is_nan() && m < f64::from(POS_EPS)
}
