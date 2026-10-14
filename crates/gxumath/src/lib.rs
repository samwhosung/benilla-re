//! The Gxu matrix builders (`0x5c3cc0` to `0x5c6620`): the perspective and orthographic projections,
//! the look-at view, the corners of a view frustum, the screen-to-world map, and the 4-vector
//! constructor. The rest of the file is `unreached` (the second look-at, the frustum planes and box,
//! the point-in-frustum test, the forward map, the resampler, the decal maps and the pickers) and is
//! not carried.
//!
//! A matrix is sixteen `f32`, row-major as `cmath` reads it (`m[4 * row + column]`); a vector of three
//! is `[f32; 3]`. The x87 runs at 53-bit precision, so an operation rounds to `f64` and a store to
//! `f32` is `as f32`. The one transcendental, `fptan` in the perspective builder, is the host's
//! tangent (`METHOD.md`).
#![forbid(unsafe_code)]

pub mod frustum;
pub mod matrix;
pub mod project;
pub mod vec;
