//! The chunk window of a light, from `LinkLightToChunks` (`0x6c0f50`), and the portal planes
//! `InitSceneTables` (`0x680610`) builds.

/// `[0x7ffab4]`: the map's half extent, 17066.666 (`0x46855555`).
pub const WORLD_GRID_MAX: f32 = f32::from_bits(0x4685_5555);
/// `[0x810ef0]`: 0.03 (`0x3cf5c28f`), units to chunks.
pub const CHUNK_SCALE_RECIP: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x86c33c]`: 0.5, subtracted before `fistp`.
pub const CHUNK_WINDOW_BIAS: f32 = 0.5;

/// `fistp` under the client's control word: round to nearest, ties to even.
fn fistp_rne(v: f64) -> i32 {
    v.round_ties_even() as i32
}

/// `fld; fsub [0x7ffab4]; fchs`: `−(c − 17066.666)` at 53 bits.
fn world_to_tile_rel(coord: f32) -> f64 {
    -(f64::from(coord) - f64::from(WORLD_GRID_MAX))
}

/// `LinkLightToChunks` (`0x6c0f50`) from `0x6c0f56` to `0x6c0fed`: the light's box (`+0x48` max x,
/// `+0x44` min x, `+0x54` max y, `+0x50` min y) as chunk indices, `fistp(f32(0.03·r) − 0.5)` with `r`
/// the distance from 17066.666. The first is multiplied in the register (`fmul st,st(1)` at
/// `0x6c0f95`); the other three are stored before their product. In store order, to `[ebp-8]`,
/// `[ebp-0x10]`, `[ebp-0x18]` and `[ebp-0xc]`.
pub fn register_light_window(max_x: f32, min_x: f32, max_y: f32, min_y: f32) -> [i32; 4] {
    let scale = f64::from(CHUNK_SCALE_RECIP);
    let bias = f64::from(CHUNK_WINDOW_BIAS);

    let c1_live = world_to_tile_rel(max_x);
    let p1 = (scale * c1_live) as f32;
    let i_maxx = fistp_rne(f64::from(p1) - bias);

    let map = |coord: f32| -> i32 {
        let tile_rel_f32 = world_to_tile_rel(coord) as f32;
        let prod = (scale * f64::from(tile_rel_f32)) as f32;
        fistp_rne(f64::from(prod) - bias)
    };
    let i_minx = map(min_x);
    let i_maxy = map(max_y);
    let i_miny = map(min_y);

    [i_maxx, i_minx, i_maxy, i_miny]
}

/// `[0x802940]`: 2.0 as `f64`.
pub const CULL_LEN2: f64 = 2.0;
/// `[0x7ffd74]`: 0.0.
pub const CULL_ZERO: f32 = 0.0;
/// `[0x8030a0]`: −1.0.
pub const CULL_NEG_ONE: f32 = -1.0;
/// `[0x7ff9d8]`: 1.0.
pub const CULL_ONE: f32 = 1.0;

/// A component times the reciprocal in the register (`fmul st,st(1)`), stored as `f32`.
fn cull_norm_component(comp: f32, inv: f64) -> f32 {
    (f64::from(comp) * inv) as f32
}

/// `InitSceneTables` (`0x680610`) from `0x680616` to `0x6807e4`: the four planes at `0xc7d0d8`, (1,
/// 0, 1), (−1, 0, 1), (0, 1, 1) and (0, −1, 1), each times `1/√2` with `√2` and its reciprocal kept
/// in the register; the first plane's x is the reciprocal itself (`fst`).
pub fn scene_cull_planes() -> [f32; 12] {
    let len = CULL_LEN2.sqrt();
    let inv = || f64::from(CULL_ONE) / len;

    let i0 = inv();
    let p0x = i0 as f32;
    let p0y = cull_norm_component(CULL_ZERO, i0);
    let p0z = cull_norm_component(CULL_ONE, i0);

    let i1 = inv();
    let p1x = cull_norm_component(CULL_NEG_ONE, i1);
    let p1y = cull_norm_component(CULL_ZERO, i1);
    let p1z = cull_norm_component(CULL_ONE, i1);

    let i2 = inv();
    let p2x = cull_norm_component(CULL_ZERO, i2);
    let p2y = cull_norm_component(CULL_ONE, i2);
    let p2z = cull_norm_component(CULL_ONE, i2);

    let i3 = inv();
    let p3x = cull_norm_component(CULL_ZERO, i3);
    let p3y = cull_norm_component(CULL_NEG_ONE, i3);
    let p3z = cull_norm_component(CULL_ONE, i3);

    [p0x, p0y, p0z, p1x, p1y, p1z, p2x, p2y, p2z, p3x, p3y, p3z]
}
