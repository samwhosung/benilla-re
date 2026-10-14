//! The ghost kit's tint and alpha, and the select screen's pet scale.

/// The float 1/255, `0x3b808081` at `0x8026c8`.
pub const INV_255: f32 = f32::from_bits(0x3b80_8081);

/// One channel of the tint of `ApplyCharSelectGhostKit` (`0x4727f0`, `0x4728ec` to `0x472939`): a
/// byte of the colour `__ftol(value)`, as `fild` times 1/255 at 53 bits rounded to `f32`. Bytes 2,
/// 1 and 0 are the red, green and blue passed to `CM2Model::SetDiffuseColor` (`0x710cf0`)
/// (`systems/glue/select.md#ghost-kit`).
pub fn color_channel(byte: u8) -> f32 {
    (f64::from(byte) * f64::from(INV_255)) as f32
}

/// The alpha of `ApplyCharSelectGhostKit` (`0x4727f0`, `0x4728a4` to `0x47293e`) passed to
/// `CM2Model::SetAlpha` (`0x710cb0`): 0.0 for a value below 0, 1.0 for a value above 1, else the value
/// unchanged, NaN and -0 included (`systems/glue/select.md#ghost-kit`).
#[allow(clippy::manual_clamp)] // the compares keep the exe's NaN and -0
pub fn clamp01(value: f32) -> f32 {
    if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

/// The pet scale of `BuildCharSelectModel` (`0x472950`, `0x472dc6`) when the pet's family has no
/// `CreatureFamily` row: `f32(a · b)` at 53 bits with `a` the model row's float at `+0x10` and `b`
/// the display row's (`systems/glue/select.md#the-pet`).
pub fn scale_product(a: f32, b: f32) -> f32 {
    (f64::from(a) * f64::from(b)) as f32
}

/// The pet scale of `BuildCharSelectModel` (`0x472950`, `0x472e73`) when the family has a row:
/// `f32(lo + (hi - lo) · (idx / total))` with `lo` and `hi` the row's minimum and maximum scales,
/// `total` the level range and `idx` the pet's level above the minimum, clamped to 0 to `total`
/// (`fild idx; fidiv total; fld hi; fsub lo; fmulp; fadd lo; fstp`, at 53 bits). A zero `total` gives
/// NaN (`systems/glue/select.md#the-pet`).
pub fn frame_interp(lo: f32, hi: f32, idx: i32, total: i32) -> f32 {
    let q = idx as f64 / total as f64;
    (q * (f64::from(hi) - f64::from(lo)) + f64::from(lo)) as f32
}
