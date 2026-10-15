//! The minimap's coordinate and blip geometry, `Minimap.cpp` (`0x6d93a0` to `0x6dab70`): the world to
//! ADT-tile map on the 64 by 64 grid, the zoom radius, the blip scale, the orientation basis and the
//! tile window. These decide where the circular map and each blip are placed (`systems/ui/minimap.md`,
//! `systems/ui/minimapblips.md`).
//!
//! The x87 runs at 53-bit precision (`docs/METHOD.md#the-harness`), so every `fmul`, `fadd`, `fsub` and
//! `fdiv` rounds to `f64` and a `fstp dword` store rounds to `f32`: `f64` arithmetic with an `as f32`
//! at the exe's own `f32` stores. Constants are the exe's `.rdata` floats by bits. A world coordinate
//! is `17066.666 - tile * 533.333`.

#![forbid(unsafe_code)]

pub mod blip;
pub mod coord;
pub mod driver;
pub mod orientation;
pub mod screen;
pub mod zoom;
