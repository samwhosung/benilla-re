//! The movement physics (`systems/collision/`): the per-frame mover, the kinematics, gravity and
//! falling, and the swept resolution against the world.
//!
//! Each function transcribes one client function, or one stretch of one, with the fields and the
//! callees' results it reads as arguments. The x87 runs at 53-bit precision, so an operation
//! rounds to `f64` and an `fstp dword` store to `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod capsule;
pub mod carrier_sweep;
pub mod frame_transform;
pub mod gravity_fall;
pub mod move_arc;
pub mod move_integrate;
pub mod move_setters;
pub mod mover;
pub mod mover_steps;
pub mod resolve_clip;
pub mod resolve_entry;
pub mod resolve_fall;
pub mod resolve_fallstep;
pub mod resolve_integrator;
pub mod resolve_modes;
pub mod resolve_multipass;
pub mod resolve_query;
pub mod resolve_slope;
pub mod resolve_state;
pub mod resolve_substep;
pub mod resolve_toolkit;
pub mod resolve_walk;
pub mod resolve_walkmode;
pub mod spline_follow;
pub mod transport_facing;
