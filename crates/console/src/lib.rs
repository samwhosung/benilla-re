//! The in-game console's text layout and animation maths of `ConsoleClient.cpp` (`0x638500` to
//! `0x63cdf0`): the font-metric derivation, the caret blink and quad, the drop-down slide, the
//! scrollbar drag, the mouse-to-line map and the display-mode aspect filter. Glyph drawing and
//! per-glyph metrics belong to the font and graphics layers and are not here.
//!
//! The x87 runs at 53-bit precision (`docs/METHOD.md#the-harness`), so every `fmul`, `fadd`, `fsub` and
//! `fdiv` rounds to `f64` and a `fstp dword` store rounds to `f32`. Each function is `f64`
//! arithmetic with an `as f32` at the exe's own `f32` stores. A comparison against a `dword`
//! operand widens it to `f64`.

#![forbid(unsafe_code)]

pub mod caret;
pub mod font_metrics;
pub mod resolution;
pub mod scale;
pub mod scroll;
pub mod slide;
