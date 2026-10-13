//! The terrain (`systems/terrain/`): the chunk meshes, strips and textures, the liquids, the
//! world queries and traces, the culling and the scene walk, and the lighting and placement
//! arithmetic around them.
//!
//! Each function transcribes one client function, or one stretch of one, with the fields and the
//! callees' results it reads as arguments. The x87 runs at 53-bit precision, so an operation
//! rounds to `f64` and an `fstp dword` store to `f32`: an `f64` operation with a final `as f32`.
//! A value kept in the register after an `fst` stays unrounded.

#![forbid(unsafe_code)]

pub mod areacoord;
pub mod boxquery;
pub mod bucketize;
pub mod camera;
pub mod chunkdraw;
pub mod clip;
pub mod collide;
pub mod colornorm;
pub mod cull;
pub mod dda;
pub mod depthbucket;
pub mod entity;
pub mod fills;
pub mod frustum;
pub mod gather;
pub mod geompred;
pub mod geomscale;
pub mod horizon;
pub mod intersect;
pub mod leaves;
pub mod lightapply;
pub mod lightblend;
pub mod liquidpass;
pub mod mapobj;
pub mod mapobjlight;
pub mod mapobjliquid;
pub mod mapobjvis;
pub mod maptriage;
pub mod mesh;
pub mod normals;
pub mod objcreate;
pub mod params;
pub mod passneg;
pub mod placement;
pub mod query;
pub mod scatter;
pub mod scenedrv;
pub mod scenevis;
pub mod scenewalk;
pub mod strips;
pub mod texgen;
pub mod texture;
pub mod waterfx;
pub mod wdlgeom;
pub mod wmovis;
pub mod worldcoord;
pub mod worldquery;
mod x87;

pub use clip::{poly_clip, unitcell_outcode, unitcube_clip};
pub use dda::{
    dda_axis_run_a, dda_axis_run_b, dda_step_xmajor, dda_step_ymajor, ray_dda_chunk, ray_dda_map,
    CellIntersector, CellList, CellSeg, StepFloats,
};
pub use fills::{
    aabb_intersect_i, chunk_mesh_tables, fx_uv_table, liquid_render_verts, mclq_verts_a,
    mclq_verts_b, quad_hole_strip_indices, wdl_mesh_fill,
};
pub use frustum::{aabb_frustum_pvertex, outcode6, sphere_frustum, volume_translate};
pub use gather::{
    chunk_collect_aabb, chunk_gather_aabb, chunk_gather_planes, mclq_collect, BatchHeader,
    BatchProtocol, ChunkModel, FacetArray, PlaneRecord,
};
pub use intersect::{
    dda_cell_intersect, dda_cell_intersect_first, CellHit, FirstHit, RaycastWorld,
};
pub use mesh::*;
pub use query::*;
pub use strips::*;
pub use texture::*;
