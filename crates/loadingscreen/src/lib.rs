//! The world-enter loading screen's geometry, `Source\LoadingScreen.cpp` (`0x4060d0` to `0x408f30`):
//! the `Glues\LoadingBar` progress fill, the letterboxed ortho rect, the taxi-route ship ribbon and
//! the `WorldMap\World%d` art tiles. Drawing, the ortho set, the fonts and the spline arc length and
//! point-at-t (`0x453360`, `0x453390`, stubbed in the tests) belong to other layers.
//!
//! The x87 runs at 53-bit precision (`docs/METHOD.md#the-harness`), so every `fmul`, `fadd`, `fsub` and
//! `fdiv` rounds to `f64` and a `fstp dword` store rounds to `f32`: `f64` arithmetic with an `as f32`
//! at the exe's own `f32` stores. Constants are the exe's `.rdata` floats by bits.

#![forbid(unsafe_code)]

pub mod bar;
pub mod ortho;
pub mod progress;
pub mod route;
pub mod worldmap;
