//! The graphics layer's maths of `WoW.exe` 1.12.1.5875: the lightning bolts ([`lightning`]) and the
//! OpenGL device's matrix, colour, viewport and rect arithmetic ([`device`]).
//!
//! The x87 runs at 53-bit precision (`METHOD.md#the-harness`), so an operation rounds to `f64` and an
//! `fstp dword` store to `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod device;
pub mod lightning;

pub use device::*;
pub use lightning::*;
