//! The models (`systems/models/`), the functions of them another system's tests run:
//! `BoxContainsTriangle` (`0x6b8b70`).
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod box_points;
