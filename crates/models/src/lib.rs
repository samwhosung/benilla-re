//! The models of the client (`systems/models/`): the M2 instance, its animation, lights and
//! render, its particle and ribbon emitters, the scene's queries, and the WMO groups, their BSP,
//! liquid and portals. Each module names the functions it covers.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]
// The exe's compares take one side on NaN; a negated compare and a clamp in the exe's order say
// which.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::manual_clamp)]

pub mod aabsp;
pub mod animation;
pub mod box_points;
pub mod cache;
pub mod emission;
pub mod lights;
pub mod liquid;
pub mod model;
pub mod particles;
pub mod portal;
pub mod render;
pub mod ribbons;
pub mod scene;
pub mod shared;
pub mod wmo;
pub mod wmo_query;
mod x87;
