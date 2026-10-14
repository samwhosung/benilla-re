//! The maths and geometry primitives other systems call: the 3×3 determinants, adjugate, inverse
//! and row scales, the 4×4 determinant, adjugate, inverse, sum, scalar product and affine
//! inverses, a matrix-to-quaternion conversion and a quaternion applied to a 4×4, ray and segment
//! tests against planes, polygons and triangles, a circle against a box, rotations about an axis
//! and from yaw, pitch and roll, the point-to-segment distance, a point and a 4-vector through a
//! 4×4 matrix, a point through a 3×4 matrix, a matrix's rows scaled, its translation shifted, the
//! 3×3, 4×4 and 3×4 products, the single-precision 4×4 product `0x754a66`, the two plane-record constructors, the conversions between a
//! colour's channels and its hue, saturation and value or its packed bytes, the cosine
//! interpolation, the reciprocal square root estimate, and the x87 float-to-integer conversions.
//!
//! Only the functions another system's crate needs or its tests run are here. The 3×3 helpers
//! are cmath's rows; the geometry functions from `0x7c0190` to `0x7c46d0` and the matrix routines
//! from `0x7bae60` to `0x7bf860` that no system claims are rows whose object file is not yet
//! identified (`unassigned`), and `0x5b7bb0` is the `net` system's. A function nothing reaches is
//! `unreached` and is not carried.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod colour;
pub mod interp;
pub mod intersect;
pub mod matrix3;
pub mod matrix34;
pub mod matrix4;
pub mod multiply;
pub mod plane;
pub mod quaternion;
pub mod raymesh;
pub mod rotation;
pub mod rsqrt;
pub mod seg_proj;
pub mod sphere_box;
pub mod transform;
pub mod x87;
