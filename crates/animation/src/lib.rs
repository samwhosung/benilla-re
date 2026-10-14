//! The M2 animation computations of WoW.exe 1.12.1.5875 (`systems/animation/`): the key search
//! and the track samplers, the bone and element evaluation of `CM2Model::Animate`, the animation
//! id resolution, `EnsureAnimated`, and the matrix and quaternion routines the pass calls.
//!
//! The x87 runs at 53 bits, so an operation rounds to `f64` and an `fstp dword` store to `f32`:
//! an `f64` operation with `as f32` at each store.

#![forbid(unsafe_code)]

pub mod animate;
pub mod bone_classify;
pub mod bone_propagate;
pub mod mat;
pub mod sample_quat;
pub mod sample_scalar;
pub mod track;
