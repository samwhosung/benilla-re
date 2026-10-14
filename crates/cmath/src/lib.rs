//! The maths and geometry primitives other systems call: the 3×3 determinant, adjugate and
//! inverse, a matrix-to-quaternion conversion, ray and segment tests against planes, polygons and
//! triangles, a circle against a box, a rotation about an axis, the point-to-segment distance, a
//! point and a 4-vector through a 4×4 matrix, a matrix's rows scaled, the 3×3 and 4×4 products,
//! a point through a 3×4 matrix, a matrix's translation shifted, and the conversions between a
//! colour's channels and its hue, saturation and value, and the reciprocal square root estimate.
//!
//! Only the functions another system's crate needs or its tests run are here. The 3×3 helpers
//! are cmath's rows; the geometry functions from `0x7c0190` to `0x7c46d0` are rows whose object
//! file is not yet identified (`unassigned`).
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod colour;
pub mod intersect;
pub mod matrix3;
pub mod multiply;
pub mod quaternion;
pub mod raymesh;
pub mod rotation;
pub mod rsqrt;
pub mod seg_proj;
pub mod sphere_box;
pub mod transform;
