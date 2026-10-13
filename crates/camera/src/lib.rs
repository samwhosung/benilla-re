//! The camera's computations (`systems/camera.md`): the angle reduction and its cosine and sine,
//! the particle density clamp, and the values the view, orthographic and unprojection setups
//! compute before handing a matrix to the graphics device.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod normalize_fp;
pub mod prim;
pub mod transform;
