//! The font and text-layout arithmetic of the client: the GxuFont, CGxFont and CGxString per-glyph
//! advance and kerning, the text measure, wrap and step, the screen-to-pixel scaling, the glyph-quad
//! geometry and atlas cell and UV arithmetic, and the glyph blits. The pixel raster is FreeType's
//! (host), the textured-quad draw belongs to the graphics layer, and the device math (projection,
//! ray, matrix) to the maths library; none of those is transcribed here. Each public item
//! transcribes one client function, and its doc-comment cites the source address.
//!
//! The x87 runs at 53-bit precision (the control word `0x027F`), so every `fmul`, `fadd`, `fsub` and
//! `fdiv` rounds to `f64` and an `fstp m32` store rounds to `f32`. Each function is transcribed as
//! `f64` arithmetic with an `as f32` at exactly the exe's f32 stores, which reproduces the double
//! rounding. Integer loads (`fild`) are transcribed as `i32 as f64`; the CRT `__ftol` (truncate toward
//! zero) is `ftol()` below.

#![forbid(unsafe_code)]

pub mod layout;
pub mod markup;
pub mod measure;
pub mod metric;
pub mod raster;
pub mod screen_pixel;
pub mod setup;

pub use benilla_re_cmath::x87::ftol;
