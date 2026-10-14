//! The aspect filter in the display-mode handler `0x63aa00`. The handler enumerates device display
//! modes and rejects one whose `width / height` (x87 `fild`, `fidiv`) is below the minimum aspect
//! `[0x804570]`, 1.248, among other gates (width at least 640, height at least 480). The rest of
//! the 936-byte handler formats strings and enumerates modes.

const MIN_ASPECT: f32 = f32::from_bits(0x3f9f_be77); // [0x804570] = 1.248

/// True when `(float)width / (float)height < minAspect` — the x87 `fcomp` C0 bit that rejects a
/// display mode (slice 0x63aa8a..0x63aa9b). `fidiv` divides the f80 width by the integer height at
/// PC_53, i.e. an f64 divide.
pub fn resolution_aspect_below_min(width: i32, height: i32) -> bool {
    let aspect = f64::from(width) / f64::from(height);
    aspect < f64::from(MIN_ASPECT)
}
