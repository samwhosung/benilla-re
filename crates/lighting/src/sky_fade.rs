//! The two cloud-cover complements `GetGlareCloudCover` (`0x6cf7b0`) and `GetGlareCoverEdge`
//! (`0x6cf7d0`) (`systems/lighting/celestial.md#glare-occlusion`). Each takes the cover sample
//! `0x6cfa90` leaves in `st0` and returns the result in `st0`, so the values are `f64`.

/// `0x6cf7b0`: `1.0 − x`, `fsubr` of the 1.0 constant `[0x7ff9d8]` from the sample.
pub fn skyblend_complement(x: f64) -> f64 {
    1.0 - x
}

/// `0x6cf7d0`: `1.0 − |2·(x − 0.5)|`, as `fsub [0x7ffa24]` (0.5), `fadd st,st`, `fabs`, `fsubr
/// [0x7ff9d8]`. The doubling is exact.
pub fn skyblend_triangle(x: f64) -> f64 {
    1.0 - (2.0 * (x - 0.5)).abs()
}
