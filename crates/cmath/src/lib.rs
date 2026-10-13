//! The maths and geometry primitives other systems call: the 3×3 determinant, adjugate and
//! inverse, a matrix-to-quaternion conversion, ray and segment tests against planes, polygons and
//! triangles, a 2×2 rotation, and the box and segment distance tests.
//!
//! Only the functions another system's crate needs or its tests run are here. The 3×3 helpers
//! are cmath's rows; the geometry functions from `0x7c0190` to `0x7c46d0` and `0x6dc900` are rows
//! whose object file is not yet identified (`unassigned`).
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod intersect;
pub mod matrix3;
pub mod quaternion;
pub mod raymesh;
pub mod rot2d;
pub mod seg_proj;
pub mod seg_seg;
pub mod sphere_box;
