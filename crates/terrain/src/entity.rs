//! The arithmetic of the world's map entities and doodad definitions: the volume test of
//! `CWorld::IsPointInEntity` (`0x670a70`), the floor drop of `LinkEntityToMapObject` (`0x6a8ed0`),
//! the chunk window of `LinkEntityToChunks` (`0x6a8ca0`), the fade of `CWorld::UpdateFadingModels`
//! (`0x672ef0`), the ground probes of `AttachEntityToGround` (`0x6a8a20`), the placement of
//! `PlaceDoodadDef` (`0x694bc0`, `systems/terrain/files.md#doodad-placements`), the colours of
//! `CreateWorldCallbackObject` (`0x670db0`) and `UpdateMapEntityEnvironment` (`0x69e280`), and the
//! probe heights of `BakeMapEntityGroupLight` (`0x69e4c0`) and `CMapDoodadDef::BakeGroupLight`
//! (`0x6a8410`).

use crate::geomscale::register_light_window;

/// 1000.0 (`0x8022cc`): the height of the floor and ground probes.
pub const FLOOR_PROBE: f32 = 1000.0;
/// 0.0005 (`0x80c698`, `0x3a03126f`): the fade per tick.
pub const DESPAWN_FADE_RATE: f32 = f32::from_bits(0x3a03_126f);
/// 1/1024 (`0x8090d8`): a placement's scale unit.
pub const MDDF_SCALE_RECIP: f32 = f32::from_bits(0x3a80_0000);
/// π/180 (`0x7ffaac`, `0x3c8efa35`).
pub const DEG2RAD: f32 = f32::from_bits(0x3c8e_fa35);
/// π (`0x81052c`, `0x40490fdb`).
pub const PI_CONST: f32 = f32::from_bits(0x4049_0fdb);

/// `0x670a70` at `0x670b3c` and four more sites: the plane offset `DotVector3` (`0x602630`)
/// returns, negated and stored as `f32`.
pub fn defvol_neg_offset(x: f32) -> f32 {
    -x
}

/// `0x670a70` from `0x670ca9` to `0x670cba`: a plane's distance `((c·pz + a·px) + b·py) + d`, kept
/// in the register for the compare with 0 at `0x670cbd`; returned stored as `f32`.
pub fn defvol_classify(plane: [f32; 4], point: [f32; 3]) -> f32 {
    let [a, b, c, d] = plane.map(f64::from);
    let [px, py, pz] = point.map(f64::from);
    (((c * pz + a * px) + b * py) + d) as f32
}

/// `0x6a8ed0` at `0x6a8f00`: the entity's height less 1000, stored as `f32`.
pub fn floordrop_down_probe(z: f32) -> f32 {
    z - FLOOR_PROBE
}

/// `0x6a8ed0` at `0x6a9093`: a hit's height plus 1000, stored as `f32`.
pub fn floordrop_offset(z: f32) -> f32 {
    z + FLOOR_PROBE
}

/// `0x6a8ed0` from `0x6a90c9` to `0x6a9128`: the point `base + (to − base)·t`. The x difference is
/// stored as `f32` before the product; y and z keep theirs in the register; each product is stored
/// as `f32` before its sum.
pub fn floordrop_lerp(base: [f32; 3], to: [f32; 3], t: f32) -> [f32; 3] {
    let t = f64::from(t);
    let dx = f64::from(to[0] - base[0]);
    let px = (dx * t) as f32;
    let py = ((f64::from(to[1]) - f64::from(base[1])) * t) as f32;
    let pz = ((f64::from(to[2]) - f64::from(base[2])) * t) as f32;
    [base[0] + px, base[1] + py, base[2] + pz]
}

/// `0x6a8ca0` from `0x6a8cbf` to `0x6a8d56`: the entity's box at `+0x44` to `+0x54` as a chunk
/// window, [`register_light_window`]'s arithmetic with the constants at `0x810a60` and `0x86aa24`,
/// which hold the same values; stored to `[ebp-0xc]`, `[ebp-0x14]`, `[ebp-0x20]` and `[ebp-0x10]`.
pub fn def_chunk_window(max_x: f32, min_x: f32, max_y: f32, min_y: f32) -> [i32; 4] {
    register_light_window(max_x, min_x, max_y, min_y)
}

/// `0x672ef0` from `0x672fd3` to `0x67302b`: a fading model's alpha. `t = (1 − age·0.0005)·w` in
/// the register, stored as `f32` without popping; below 0 (the unrounded value, ordered, `jp` at
/// `0x672ff3`) it is 0, above 1 (the stored value, `jne` at `0x67300c`) it is 1, and otherwise, NaN
/// included, `(3 − 2t)·t·t` from the stored `t`.
pub fn doodad_despawn_fade(age: i32, weight: f32) -> f32 {
    let live = (1.0 - f64::from(age) * f64::from(DESPAWN_FADE_RATE)) * f64::from(weight);
    if live < 0.0 {
        return 0.0;
    }
    let t = f64::from(live as f32);
    if t > 1.0 {
        1.0
    } else {
        ((3.0 - (t + t)) * t * t) as f32
    }
}

/// `0x6a8a20` at `0x6a8a54`: the entity's height less 1000, stored as `f32`.
pub fn cmapdoodaddef_ground_down(z: f32) -> f32 {
    z - FLOOR_PROBE
}

/// `0x6a8a20` from `0x6a8ab8` to `0x6a8add`: `(src_z + 1000) + probe_z`, the first sum kept in the
/// register, stored as `f32`.
pub fn cmapdoodaddef_ground_up(src_z: f32, probe_z: f32) -> f32 {
    ((f64::from(src_z) + f64::from(FLOOR_PROBE)) + f64::from(probe_z)) as f32
}

/// One colour component to a byte as `0x670db0` and `0x69e280` do: 0 at or below 0 (`jp` taken
/// only above 0 or on NaN), 255 at or above 1 (`jne` taken below 1 or on NaN), otherwise `__ftol`
/// of `c·255 + 0.5`; NaN takes that path and gives 0.
fn component_byte(c: f32) -> u32 {
    let c = f64::from(c);
    if c <= 0.0 {
        0
    } else if c >= 1.0 {
        255
    } else {
        crate::x87::ftol(c * 255.0 + 0.5) as u32 & 0xff
    }
}

/// The colour words of `0x670db0` and `0x69e280`: `0xff000000 | r << 16 | g << 8 | b` from three
/// components in `[0, 1]`, stored at the object's `+0xf4`.
pub fn clamp01_to_color(src: [f32; 3]) -> u32 {
    let [r, g, b] = src.map(component_byte);
    0xff00_0000 | (r << 16) | (g << 8) | b
}

/// The placement record fields `0x694bc0` reads.
#[derive(Clone, Copy, Debug)]
pub struct MddfRecord {
    /// `+0x8`.
    pub y: f32,
    /// `+0xc`.
    pub z: f32,
    /// `+0x10`.
    pub x: f32,
    /// `+0x14`, degrees.
    pub rot_x_deg: f32,
    /// `+0x18`, degrees.
    pub rot_y_deg: f32,
    /// `+0x1c`, degrees.
    pub rot_z_deg: f32,
    /// `+0x20`, in 1024ths.
    pub scale_u16: u16,
}

/// What `0x694bc0` derives from a placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MddfXform {
    /// `+0x24`: the position.
    pub pos: [f32; 3],
    /// `+0x30`: the scale.
    pub scale: f32,
    /// `[ebp-0x14]`: the z angle.
    pub rot_z: f32,
    /// `[ebp-0x10]`: the x angle.
    pub rot_x: f32,
    /// `[ebp-0xc]`: the y angle, plus π.
    pub rot_y: f32,
}

/// `0x694bc0` from `0x694c8b` to `0x694d55`: the position `(−x, −y, z)` plus the tile origin, each
/// negation and sum stored as `f32`; the scale `u16/1024`; the angles in radians, the y angle plus
/// π in the register.
pub fn mddf_create_xform(rec: &MddfRecord, tile_origin: [f32; 3]) -> MddfXform {
    let pos = [
        -rec.x + tile_origin[0],
        tile_origin[1] + -rec.y,
        tile_origin[2] + rec.z,
    ];
    let rad = |deg: f32| f64::from(deg) * f64::from(DEG2RAD);
    MddfXform {
        pos,
        scale: (f64::from(rec.scale_u16) * f64::from(MDDF_SCALE_RECIP)) as f32,
        rot_z: rad(rec.rot_z_deg) as f32,
        rot_x: rad(rec.rot_x_deg) as f32,
        rot_y: (rad(rec.rot_y_deg) + f64::from(PI_CONST)) as f32,
    }
}

/// `0x69e4c0` at `0x69e4f8` and `0x69e515`: a height plus 1 and less 12, each stored as `f32`.
pub fn cmapobj_vert_offsets(z: f32) -> (f32, f32) {
    (z + 1.0, z - 12.0)
}

/// `0x6a8410` at `0x6a8436` and `0x6a8475`: a height plus 1.5 and less 1760, each stored as `f32`.
pub fn cmapobj_skyocc_offsets(z: f32) -> (f32, f32) {
    (z + 1.5, z - 1760.0)
}
