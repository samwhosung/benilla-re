//! The scale arithmetic of `CSimpleHyperlinkButton::Configure` (`0x7a3350`,
//! `systems/ui/hyperlinks.md#the-pool`).

/// `0x7ff9d8`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);

/// `0x7a341a` to `0x7a343c`: `R = 1.0 / scale` with `scale` the widget's `+0x7c`, and the products
/// `A * R` and `R * B`. `R` is an f64 left live across both products; only the stores narrow to
/// `f32`. Returns the stores at `[esp+4]` and `[esp]`.
pub fn recip_scale_block(a: f32, b: f32, scale: f32) -> (f32, f32) {
    let r = f64::from(ONE) / f64::from(scale);
    ((f64::from(a) * r) as f32, (r * f64::from(b)) as f32)
}

/// `0x7a3446` to `0x7a3457` and `0x7a345a` to `0x7a346b`: `(d - s) / scale` stored as an `f32`.
pub fn axis_offset(d: f32, s: f32, scale: f32) -> f32 {
    ((f64::from(d) - f64::from(s)) / f64::from(scale)) as f32
}
