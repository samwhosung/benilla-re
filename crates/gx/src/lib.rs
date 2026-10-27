//! The graphics layer's maths of `WoW.exe` 1.12.1.5875: the lightning bolts ([`lightning`]) and the
//! OpenGL device's matrix, colour, viewport and rect arithmetic ([`device`]), the Direct3D
//! device's texture coordinate modes ([`texmode`]), the texture layer's size and format
//! decisions and TGA pixel copy ([`textures`]) and the window procedures' drag rectangle
//! ([`window`]).
//!
//! The x87 runs at 53-bit precision (`docs/METHOD.md#the-harness`), so an operation rounds to `f64` and an
//! `fstp dword` store to `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod device;
pub mod lightning;
pub mod texlevels;
pub mod texmode;
pub mod textures;
pub mod window;

pub use device::*;
pub use lightning::*;
