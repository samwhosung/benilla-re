//! The arithmetic of the client's spell code (`systems/spell/`): the range and radius gates, the
//! cast time and power cost scaling, the modifier applier, the effect amount's integer formula
//! and the spell visuals' timing and scaling.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with an `as f32` at each store.

#![forbid(unsafe_code)]

pub mod cast;
pub mod cooldown;
pub mod visual;
