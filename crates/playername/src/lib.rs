//! The overhead world text of `PlayerName.cpp`, `Source\Game\GameClient` (the unit names, the
//! damage and heal numbers and the raid-target icons): the anchor lift, the swing-trail cadence, the
//! per-category interpolation, keyframe and fade timing, and the billboard, quad and matrix geometry.
//! The rendering, the projection and the matrix build belong to the graphics layer, the maths library
//! and the interface and are not here. Each function is compared with the exe's own code in
//! `tests/`.
pub mod anchor;
pub mod clip_quad;
pub mod plate_matrix;
pub mod render;
pub mod swing;
pub mod timing;
