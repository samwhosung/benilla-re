//! The placement arithmetic of `PlaceMapObjDef` (`0x695650`,
//! `systems/terrain/files.md#object-placements`), `AnchorPlacement` (`0x698110`) and the fade of
//! `CMapDoodadDef::Repose` (`0x6a81d0`).

use crate::x87::fistp;

/// `[0x7ffaac]`: π/180 (`0x3c8efa35`).
pub const DEG_TO_RAD: f32 = f32::from_bits(0x3c8e_fa35);
/// `[0x81052c]`: π (`0x40490fdb`).
pub const PI_F32: f32 = f32::from_bits(0x4049_0fdb);
/// `[0x7ffa24]`: 0.5.
pub const HALF: f32 = 0.5;
/// `[0x810678]`: 4.1666665 (`0x40855555`), a cell's width.
pub const GRID_STEP: f32 = f32::from_bits(0x4085_5555);
/// `[0x810674]`: 0.24 (`0x3e75c28f`), cells per unit.
pub const GRID_INV: f32 = f32::from_bits(0x3e75_c28f);
/// `[0x86a6e4]`: 0.5, subtracted before `fistp`.
pub const QUANT_BIAS: f32 = 0.5;

/// `PlaceMapObjDef` (`0x695650`) from `0x6956e5` to `0x69571d`: `[+0x10]` and `[+0x8]` of the entry
/// negated and stored, `[+0xc]` copied, then the origin added to each, stored to `this+0x24` up:
/// `(−v0 + o.x, o.y + −v1, o.z + v2)` with `v0 = [+0x10]`, `v1 = [+0x8]`, `v2 = [+0xc]`.
pub fn placement_origin_xform(v0: f32, v1: f32, v2: f32, base: [f32; 3]) -> [f32; 3] {
    basis_row_xform(v0, v1, v2, base)
}

/// `0x695650` at `0x695720` and `0x69572c`: `f32([+0x1c]·π/180)` and the same of `[+0x14]`.
pub fn euler_deg_to_rad(angle: f32) -> f32 {
    (f64::from(angle) * f64::from(DEG_TO_RAD)) as f32
}

/// `0x695650` at `0x695738`: `f32([+0x18]·π/180 + π)`, the sum in the register.
pub fn euler_deg_to_rad_plus_pi(angle: f32) -> f32 {
    ((f64::from(angle) * f64::from(DEG_TO_RAD)) + f64::from(PI_F32)) as f32
}

/// `0x695650` from `0x695854` and from `0x695887`: a box corner as the origin is, `(−a + o.x, o.y +
/// −b, o.z + c)`, the negations stored first; `a, b, c` are `[+0x34], [+0x2c], [+0x24]` for the first
/// corner (`this+0x44`) and `[+0x28], [+0x20], [+0x30]` for the second (`this+0x50`).
pub fn basis_row_xform(a: f32, b: f32, c: f32, base: [f32; 3]) -> [f32; 3] {
    let na = (-f64::from(a)) as f32;
    let nb = (-f64::from(b)) as f32;
    [
        (f64::from(na) + f64::from(base[0])) as f32,
        (f64::from(base[1]) + f64::from(nb)) as f32,
        (f64::from(base[2]) + f64::from(c)) as f32,
    ]
}

/// The sphere `0x695650` sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundSphere {
    /// `this+0x5c`: the corners' midpoint.
    pub center: [f32; 3],
    /// `this+0x68`: the second corner's distance from it.
    pub radius: f32,
}

/// `0x695650` from `0x6958bc` to `0x695931`: the centre `(p2 + p1)·0.5`, x and y halved in the
/// register and z stored to `[ebp-0x14]` before (`0x6958ce`); the radius `√((dz² + dy²) + dx²)` of
/// `p2` less the stored centre, all in the register.
pub fn bound_sphere(p1: [f32; 3], p2: [f32; 3]) -> BoundSphere {
    let sx = f64::from(p2[0]) + f64::from(p1[0]);
    let sy = f64::from(p2[1]) + f64::from(p1[1]);
    let sz = (f64::from(p2[2]) + f64::from(p1[2])) as f32;
    let cx = (sx * f64::from(HALF)) as f32;
    let cy = (sy * f64::from(HALF)) as f32;
    let cz = (f64::from(sz) * f64::from(HALF)) as f32;
    let dx = f64::from(p2[0]) - f64::from(cx);
    let dy = f64::from(p2[1]) - f64::from(cy);
    let dz = f64::from(p2[2]) - f64::from(cz);
    BoundSphere {
        center: [cx, cy, cz],
        radius: ((dz * dz + dy * dy) + dx * dx).sqrt() as f32,
    }
}

/// `AnchorPlacement` (`0x698110`) from `0x69821f`: the model box's corners added, each sum stored.
pub fn bounds_sum(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|k| (f64::from(a[k]) + f64::from(b[k])) as f32)
}

/// `0x698110` from `0x698247`: each sum times 0.5, stored.
pub fn bounds_center(sum: [f32; 3]) -> [f32; 3] {
    sum.map(|c| (f64::from(c) * f64::from(HALF)) as f32)
}

/// `0x698110` from `0x698278`: a corner less the centre, stored.
pub fn recenter(p: [f32; 3], center: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|k| (f64::from(p[k]) - f64::from(center[k])) as f32)
}

/// `0x698110` from `0x6982d4`: the rotated point plus the position, stored.
pub fn world_add(r: [f32; 3], pos: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|k| (f64::from(r[k]) + f64::from(pos[k])) as f32)
}

/// `0x698110` at `0x6982fb` (x) and `0x698319` (y): the cell `fistp(f32((a + 4.1666665)·0.24) −
/// 0.5)`, the product stored to `[ebp+8]` and reloaded.
pub fn grid_index(anchor: f32) -> i32 {
    let prod = ((f64::from(anchor) + f64::from(GRID_STEP)) * f64::from(GRID_INV)) as f32;
    fistp(f64::from(prod) - f64::from(QUANT_BIAS))
}

/// `0x698110` at `0x698337` (x) and `0x698348` (y): `pos − (anchor − idx·4.1666665)` (`fild; fmul;
/// fsubr; fsubr`) in the register, stored over the position.
pub fn grid_remainder(idx: i32, anchor: f32, pos: f32) -> f32 {
    let after_anchor = f64::from(anchor) - f64::from(idx) * f64::from(GRID_STEP);
    (f64::from(pos) - after_anchor) as f32
}

/// `CMapDoodadDef::Repose` (`0x6a81d0`) from `0x6a83b4`: `[ebx+0x68] = f32([ebp-0x1c]·[ebx+0x30])`,
/// the bytes of `crate::lightblend::chunk_fade_scale` (`0x6952a0`).
pub fn chunk_fade_scale_twin(fade: f32, scale: f32) -> f32 {
    (f64::from(fade) * f64::from(scale)) as f32
}
