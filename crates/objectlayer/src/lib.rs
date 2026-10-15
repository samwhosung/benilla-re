//! The object layer's arithmetic, transcribed from the client and held to its bytes by the tests:
//! the update-field and movement block codecs, the monster path and its anchor, the move events,
//! the facing, the faction and reaction tables, the transport and missile paths, and the float
//! kernels of the unit, player, game object, corpse and combat log code.
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`. Each kernel's header names the instructions it
//! stands for and the doc entry it supports; [`x87`] holds the conversions and the compare that
//! several of them share.

#![forbid(unsafe_code)]

pub mod apply;
pub mod cast_time;
pub mod datastore;
pub mod dynobj_fp;
pub mod dynoffset;
pub mod facing;
pub mod faction;
pub mod gameobject;
pub mod gobj1_fp;
pub mod gobj2_fp;
pub mod gobj3_fp;
pub mod leg_timing;
pub mod missile;
pub mod missile_flight;
pub mod monster_move;
pub mod move_events;
pub mod movement;
pub mod objclass_fp;
pub mod objf2_fp;
pub mod objf3_fp;
pub mod objf_fp;
pub mod objmisc_fp;
pub mod objtail_fp;
pub mod player2_fp;
pub mod player3_fp;
pub mod player_fp;
pub mod spline_anchor;
pub mod unit_fp;
pub mod unitbig1_fp;
pub mod unitbig2_fp;
pub mod unitbig3_fp;
pub mod unith_fp;
pub mod unitm1_fp;
pub mod unitm2_fp;
pub mod unitm3_fp;
pub mod unitm4_fp;
pub mod unitm5_fp;
pub mod update;
pub mod updatefields;
pub mod vector3;
pub mod x87;
