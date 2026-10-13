//! `SysMessage.cpp`'s one computation: a system message type's colour, normalized.
//!
//! The x87 runs at 53-bit precision: `fild` widens the byte, `fmul` rounds to `f64`, and the
//! `fstp m32` store rounds to `f32`, so the arithmetic is `f64` with a final `as f32`.

#![forbid(unsafe_code)]

/// `[0x8026c8]`, `1/255` as `f32`.
pub const INV_255: f32 = f32::from_bits(0x3b80_8081);

/// One channel of `0x44cca0`: `(float)byte * (1/255)` (`fild; fmul [0x8026c8]; fstp`), applied
/// to the R, G and B bytes of the colour record at the message type's index.
pub fn color_channel(byte: u8) -> f32 {
    (f64::from(byte) * f64::from(INV_255)) as f32
}
