//! The arithmetic the client does around its sound engine (`systems/sound/`): the volume levels
//! and rates it hands to FMOD, the world-to-FMOD axis remap, the distance gates and gains, the
//! per-play volume and pitch variation, and the emitter searches. FMOD itself (`fmod.dll`) is
//! outside the exe and outside this crate.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`. Vectors are `[f32; 3]` as the client stores
//! them.

#![forbid(unsafe_code)]

mod spatial;
mod variation;
mod volume;

pub use spatial::*;
pub use variation::*;
pub use volume::*;
