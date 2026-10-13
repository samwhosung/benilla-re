//! Object-layer computations that the movement code's tests reach: the monster path's anchor and
//! its reader, the packed path offset, the move-event tests and the facing arithmetic.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

pub mod facing;
pub mod monster_move;
pub mod move_events;
pub mod spline_anchor;
