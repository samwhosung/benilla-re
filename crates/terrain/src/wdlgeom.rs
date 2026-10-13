//! The low-detail tile geometry of `CMap::LoadWdl` (`0x6944a0`, `systems/terrain/files.md#wdl`), the
//! light arithmetic of `CreateGroupLights` (`0x695c00`) and the camera height of
//! `UpdateCameraEnvironment` (`0x6809c0`).

/// `[0x80fee4]`: 33.333332 (`0x42055555`).
pub const CHUNK_SIZE: f32 = f32::from_bits(0x4205_5555);
/// `[0x8105b0]`: −33.333332 (`0xc2055555`).
pub const CHUNK_SIZE_NEG: f32 = f32::from_bits(0xc205_5555);
/// `[0x7ffab4]`: 17066.666 (`0x46855555`).
pub const WORLD_GRID_MAX: f32 = f32::from_bits(0x4685_5555);
/// `[0x80654c]`: 533.3333 (`0x44055555`), a tile's width.
pub const TILE_SIZE: f32 = f32::from_bits(0x4405_5555);
/// `[0x7ffa24]`: 0.5.
pub const HALF: f32 = 0.5;
/// `[0x801360]`: 0.001 (`0x3a83126f`).
pub const SLOPE_EPS: f32 = f32::from_bits(0x3a83_126f);
/// `[0x801620]`: 0.1 (`0x3dcccccd`).
pub const SLOPE_BUMP: f32 = f32::from_bits(0x3dcc_cccd);
/// `[0x8026c8]`: 1/255 (`0x3b808081`).
pub const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// `[0x7ff9d8]`: 1.0.
pub const RECIP_ONE: f32 = 1.0;

/// What `0x6944a0` writes for a tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WdlTileGeom {
    /// `+0x0`: the y corner less a tile.
    pub min_y: f32,
    /// `+0x4`: the x corner less a tile.
    pub min_x: f32,
    /// `+0xc` and `+0x28`: `17066.666 − row·33.333332`.
    pub origin_y: f32,
    /// `+0x10` and `+0x2c`: `col·−33.333332 + 17066.666`.
    pub origin_x: f32,
    /// `+0x14`: the highest height.
    pub max_z: f32,
    /// `+0x18`: the box's centre.
    pub center: [f32; 3],
    /// `+0x24`: the distance from the centre to the high corner.
    pub radius: f32,
}

/// `CMap::LoadWdl` (`0x6944a0`) from `0x694648`, the column `ix` and row `iy` loaded with `fild`, the
/// lowest height and the highest (in the register, an integer): the corners stored; the low corner
/// differences kept in the register for the sums, the y sum stored before its half; the centre's
/// components stored; and `√((ez² + ey²) + ex²)` from the stored corner and centre.
pub fn wdl_tile_geometry(ix: i32, iy: i32, min_z: f32, max_z: f32) -> WdlTileGeom {
    let half = f64::from(HALF);
    let tile = f64::from(TILE_SIZE);
    let origin_x = (f64::from(ix) * f64::from(CHUNK_SIZE_NEG) + f64::from(WORLD_GRID_MAX)) as f32;
    let origin_y = (-(f64::from(iy) * f64::from(CHUNK_SIZE)) + f64::from(WORLD_GRID_MAX)) as f32;
    let min_y_live = f64::from(origin_y) - tile;
    let min_x_live = f64::from(origin_x) - tile;
    let sum_y = (min_y_live + f64::from(origin_y)) as f32;
    let sum_x = min_x_live + f64::from(origin_x);
    let sum_z = f64::from(min_z) + f64::from(max_z);
    let center = [
        (f64::from(sum_y) * half) as f32,
        (sum_x * half) as f32,
        (sum_z * half) as f32,
    ];
    let ex = f64::from(origin_y) - f64::from(center[0]);
    let ey = f64::from(origin_x) - f64::from(center[1]);
    let ez = f64::from(max_z) - f64::from(center[2]);
    WdlTileGeom {
        min_y: min_y_live as f32,
        min_x: min_x_live as f32,
        origin_y,
        origin_x,
        max_z,
        center,
        radius: ((ez * ez + ey * ey) + ex * ex).sqrt() as f32,
    }
}

/// `CreateGroupLights` (`0x695c00`) at `0x695cb1`: `|B − A|` of `[esi+0x2c]` and `[esi+0x28]` against
/// 0.001, `test ah,5; jp`: below it B is raised by 0.1.
pub fn detail_slope_bumped(a: f32, b: f32) -> bool {
    (f64::from(b) - f64::from(a)).abs() < f64::from(SLOPE_EPS)
}

/// `0x695c00` from `0x695c93` to `0x695ce2`: `[edi+0xec]` is B, or `f32(B + 0.1)` when
/// `detail_slope_bumped`; `[edi+0xf0]` is `1/([edi+0xec] − A)`.
pub fn detail_slope_recip(a: f32, b: f32) -> f32 {
    let val = if detail_slope_bumped(a, b) {
        (f64::from(b) + f64::from(SLOPE_BUMP)) as f32
    } else {
        b
    };
    (f64::from(RECIP_ONE) / (f64::from(val) - f64::from(a))) as f32
}

/// `0x695c00` from `0x695d34` and from `0x695da3`: the colour bytes times 1/255 times the scale, the
/// first kept in the register and the other two stored first; stored as `[b, g, r]`.
pub fn detail_color_scale(r: u8, g: u8, b: u8, scale: f32) -> [f32; 3] {
    let inv = f64::from(INV_255);
    let sc = f64::from(scale);
    let g_norm = (f64::from(g) * inv) as f32;
    let b_norm = (f64::from(b) * inv) as f32;
    [
        (f64::from(b_norm) * sc) as f32,
        (f64::from(g_norm) * sc) as f32,
        (f64::from(r) * inv * sc) as f32,
    ]
}

/// `UpdateCameraEnvironment` (`0x6809c0`) at `0x680a81` and `0x680aaa`: `a − b` to `[0xc7bc60]`.
pub fn camera_probe_z_delta(a: f32, b: f32) -> f32 {
    (f64::from(a) - f64::from(b)) as f32
}
