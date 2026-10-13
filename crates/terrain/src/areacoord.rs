//! The arithmetic of three area queries: the tile window of `FindNearestLiquidInAreas`
//! (`0x68af10`), the downward ray of `UpdateShowQuery` (`0x6821f0`) and that of
//! `CWorld::GetAreaIdAt` (`0x670250`), with the compare each uses to pick the nearer hit.

use crate::x87::fistp;

/// `[0x7ffab4]`: the map's half extent, 17066.666 (`0x46855555`).
pub const MAP_CENTER_F32: f32 = f32::from_bits(0x4685_5555);

/// `[0x8102a0]`: world units to cells, 0.24 (`0x3e75c28f`).
pub const AREA_SCALE: f32 = f32::from_bits(0x3e75_c28f);

/// `[0x869ffc]`: 0.5, subtracted before each `fistp`.
const AREA_BIAS: f32 = 0.5;

/// `[0x810284]`: 533.3333 (`0x44055555`), a tile's edge.
pub const AREA_PARAM_OFFSET: f32 = f32::from_bits(0x4405_5555);

/// `0x68af1b` to `0x68af60`, x: `fistp(f32(0.24·f32(−(px − C))) − 0.5)`; the negation is stored
/// before the product.
pub fn area_coord_x(px: f32) -> i32 {
    let a = (-(f64::from(px) - f64::from(MAP_CENTER_F32))) as f32;
    let c = (f64::from(AREA_SCALE) * f64::from(a)) as f32;
    fistp(f64::from(c) - f64::from(AREA_BIAS))
}

/// The same stretch, y: `fistp(f32(−(py − C)·0.24) − 0.5)`, one store.
pub fn area_coord_y(py: f32) -> i32 {
    let b = ((-(f64::from(py) - f64::from(MAP_CENTER_F32))) * f64::from(AREA_SCALE)) as f32;
    fistp(f64::from(b) - f64::from(AREA_BIAS))
}

/// `0x68af77`: the tile column, `(ix >> 7) & 0x3f` with an arithmetic shift.
pub fn area_tile_x(px: f32) -> i32 {
    (area_coord_x(px) >> 7) & 0x3f
}

/// `0x68af89`: the tile row, `(iy >> 7) & 0x3f`.
pub fn area_tile_y(py: f32) -> i32 {
    (area_coord_y(py) >> 7) & 0x3f
}

/// `0x68af9f` to `0x68afd5`: the tiles either side, `(max(t − 1, 0), min(t + 1, 0x3f))`, both
/// inclusive (`setle; dec; and` and a signed `jl`).
pub fn area_window(t: i32) -> (i32, i32) {
    let tm1 = t - 1;
    let lo = if tm1 <= 0 { 0 } else { tm1 };
    let tp1 = t + 1;
    let hi = if tp1 < 0x3f { tp1 } else { 0x3f };
    (lo, hi)
}

/// `0x68b031` to `0x68b05a`: an area's `+0x7c` or `+0x80` less 533.3333, stored as `f32`, the two
/// values `0x68b0d0` receives.
pub fn area_param_offset(field: f32) -> f32 {
    (f64::from(field) - f64::from(AREA_PARAM_OFFSET)) as f32
}

/// `[0x80fec8]`: 1760.0, the drop of the downward ray.
pub const RAY_DROP_Z: f32 = 1760.0;

/// `0x68223c`: the ray's end in `0x6821f0`, `f32(eye_z − 1760)`, the eye's z copied from
/// `0xc7cf28`.
pub fn ray_end_z(eye_z: f32) -> f32 {
    (f64::from(eye_z) - f64::from(RAY_DROP_Z)) as f32
}

/// `0x6822a2`: the byte returned by the map object query is cleared when the terrain's fraction is
/// below the object's (`test ah,5; jp`); equal or NaN keep it.
pub fn nearer_select_jp(entry_al: u8, terrain_t: f32, wmo_t: f32) -> u8 {
    if terrain_t < wmo_t {
        0
    } else {
        entry_al
    }
}

/// `[0x808300]`: 0.33 (`0x3ea8f5c3`), the lift of the ray's start.
pub const RAY_LIFT_Z: f32 = f32::from_bits(0x3ea8_f5c3);

/// `0x6702a2`: the ray's start in `0x670250`, `f32(pos_z + 0.33)`.
pub fn area_ray_start_z(pos_z: f32) -> f32 {
    (f64::from(pos_z) + f64::from(RAY_LIFT_Z)) as f32
}

/// `0x6702c1`: the ray's end in `0x670250`, `f32(pos_z − 1760)`.
pub fn area_ray_end_z(pos_z: f32) -> f32 {
    (f64::from(pos_z) - f64::from(RAY_DROP_Z)) as f32
}

/// `0x670345`: the map object's hit is kept unless the terrain's fraction is below it (`test ah,5;
/// jnp`); equal or NaN keep it.
pub fn nearer_keep_wmo_jnp(terrain_t: f32, wmo_t: f32) -> bool {
    terrain_t.partial_cmp(&wmo_t) != Some(std::cmp::Ordering::Less)
}
