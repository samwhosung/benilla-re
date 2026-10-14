//! The curve evaluators of the movement spline file (`0x453300` to `0x454980`) and the vector
//! leaves after them: the basis polynomial and the point evaluators over four control points, the
//! chord-sum arc length, the segment locators, the knot sums, the position, length and tangent
//! frame of a segment, the squared lengths of a vector of two and of three, and the division of a
//! vector by a scalar. `0x454a10` (the integer square root) and `0x456280` (the normalization of
//! a vector of two) belong to object files not yet identified (`unassigned`).
//!
//! Only the functions something reaches are here; the degree-1 evaluator `0x4536d0` is `unreached`
//! and is not carried. The x87 runs at 53-bit precision, so an operation rounds to `f64` and a
//! store to `f32` is `as f32`: each function says which intermediate it keeps in the register and
//! which it stores. The constants the code reads are passed in by the caller where the function
//! reads them from the image.
#![allow(clippy::float_cmp)]
#![forbid(unsafe_code)]

pub mod eval;
pub mod frame;
pub mod scalar;
pub mod vector;
