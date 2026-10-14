//! The ground grid the weather particles land on, and the mist puff placement: the cell height
//! `0x67c820` (`systems/lighting/weather.md#ground-grid`), the grid height `0x67ca00`, the cell
//! height by index `0x67ca90`, the segment walk `0x67cb60`, and `0x67a990` PlacePuff
//! (`systems/lighting/precipitation.md#mist`).
//!
//! The grid object has the table `0x810038` and a 5 by 5 window of tile pointers at `+0x28` (grid
//! indices 10 to 34, `col + 5 * (row + 2)`). The tile coordinates of its first column and row are
//! `+0x18` and `+0x1c`, and those times 32 are `+0x20` and `+0x24`. A tile is 0x1008 bytes: 32 by
//! 32 `f32` cells, then the tile coordinates at `+0x1000` and `+0x1004`. An unfilled cell holds
//! FLT_MAX (`[0x80ff70]`) and the first sample fills it through the adapter `0x67c7f0` (`0x6b7070`
//! with 200, else z less 200), which the functions here take as a closure.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`; the double handed to
//! `floor` (`0x73ff3f`) is a `fstp qword` and keeps all 53 bits.

use benilla_re_cmath::multiply::multiply_matrix4 as mat4_mul;
use benilla_re_cmath::rotation::axis_angle_mat4;
use benilla_re_cmath::transform::transform_mat43_vec3;
use benilla_re_random::Stream;

use crate::weather_kernels::weather_fixed_quantize;
use crate::x87::ftol_floor;

/// `[0x80ff70]`: the unfilled-cell value, and what every point outside the window returns.
pub const FLT_MAX_SENTINEL: f32 = f32::from_bits(0x7f7f_ffff);
/// `[0x80ff8c]` = 0.96: the cell scale of `0x67c820`.
const Q_SCALE: f32 = f32::from_bits(0x3f75_c28f);
/// `[0x8029cc]` = 512.0: the bias that puts the cell in bits 14 to 21 of the stored f32.
const Q_BIAS: f32 = f32::from_bits(0x4400_0000);
/// `[0x80ff90]` = 0.03: the tile scale of `0x67ca00`.
const TILE_SCALE_SAMPLE: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x80ff94]` = 0.96: the cell scale of `0x67cb60`.
const TILE_SCALE_MARCH: f32 = f32::from_bits(0x3f75_c28f);
/// `[0x80fee4]` = 33.33333: the world size of a tile.
const CELL_TO_WORLD: f32 = f32::from_bits(0x4205_5555);
/// `[0x80ff98]` = 1.0416666: the world size of a cell.
const STRIDE_CELL: f32 = f32::from_bits(0x3f85_5555);
/// `[0x7ffa24]` = 0.5.
const HALF: f32 = f32::from_bits(0x3f00_0000);

/// One tile of the window: its coordinates (`+0x1000`, `+0x1004`) and its cells.
#[derive(Clone)]
pub struct Tile {
    /// The tile coordinates.
    pub coord: (i32, i32),
    /// The 32 by 32 cells, row-major.
    pub cells: [f32; 0x400],
}

/// `0x67c820` GetWeatherTileHeight (`systems/lighting/weather.md#ground-grid`), the whole function
/// on one tile and the world point `(x, y)`: `fx = x - tile.x * 33.33333` and `fy` likewise are
/// stored as f32; `fcomp 0.0; test ah,5; jnp` returns FLT_MAX for either below 0 (NaN and -0 pass).
/// The cell is `(bits(f32(0.96 * f + 512)) >> 14) & 0xff` per axis, and FLT_MAX is returned when
/// either is 0x20 or more. A cell equal to FLT_MAX is filled with `terrain_height`; the cell is
/// returned.
pub fn tile_cell_fill(
    cells: &mut [f32; 0x400],
    tile_coord: (i32, i32),
    world: (f32, f32),
    mut terrain_height: impl FnMut(f32, f32) -> f32,
) -> f32 {
    let fx = (f64::from(world.0) - f64::from(tile_coord.0) * f64::from(CELL_TO_WORLD)) as f32;
    let fy = (f64::from(world.1) - f64::from(tile_coord.1) * f64::from(CELL_TO_WORLD)) as f32;
    if fx < 0.0 || fy < 0.0 {
        return FLT_MAX_SENTINEL;
    }
    let bias_y = (f64::from(Q_SCALE) * f64::from(fy) + f64::from(Q_BIAS)) as f32;
    let bias_x = (f64::from(Q_SCALE) * f64::from(fx) + f64::from(Q_BIAS)) as f32;
    let cy = (bias_y.to_bits() >> 0xe) & 0xff;
    let cx = (bias_x.to_bits() >> 0xe) & 0xff;
    if cy >= 0x20 || cx >= 0x20 {
        return FLT_MAX_SENTINEL;
    }
    let idx = (cy * 0x20 + cx) as usize;
    if cells[idx].to_bits() == FLT_MAX_SENTINEL.to_bits() {
        cells[idx] = terrain_height(world.0, world.1);
    }
    cells[idx]
}

/// What [`mist_grid_sample`] returns.
pub struct GridSampleOut {
    /// The cell value, or FLT_MAX outside the window.
    pub value: f32,
    /// The window index `tx + 5 * (ty + 2)` of the tile sampled, `None` outside the window.
    pub window_index: Option<usize>,
}

/// `0x67ca00` GetWeatherGridHeight (`systems/lighting/weather.md#ground-grid`), the whole function
/// on the point `world` (`ecx` the grid, `cell_origin` its `+0x18` and `+0x1c`, `tiles` its window
/// by `index - 10`). The tile is `floor(0.03 * p) - origin` per axis; the y product is stored as
/// f32 (`fstp dword [ebp+8]`) before the floor and the x product is not. Outside 0 to 4 on either
/// axis the result is FLT_MAX; otherwise it is [`tile_cell_fill`] on tile `tx + 5 * (ty + 2)`.
pub fn mist_grid_sample(
    cell_origin: (i32, i32),
    world: (f32, f32),
    tiles: &mut [Tile],
    terrain_height: impl FnMut(f32, f32) -> f32,
) -> GridSampleOut {
    let ty_scaled = (f64::from(TILE_SCALE_SAMPLE) * f64::from(world.1)) as f32;
    let tx = ftol_floor(f64::from(TILE_SCALE_SAMPLE) * f64::from(world.0)) - cell_origin.0;
    let ty = ftol_floor(f64::from(ty_scaled)) - cell_origin.1;
    if !(0..5).contains(&tx) || !(0..5).contains(&ty) {
        return GridSampleOut {
            value: FLT_MAX_SENTINEL,
            window_index: None,
        };
    }
    let idx = (tx + 5 * (ty + 2)) as usize;
    let tile = &mut tiles[idx - 10];
    let value = tile_cell_fill(&mut tile.cells, tile.coord, world, terrain_height);
    GridSampleOut {
        value,
        window_index: Some(idx),
    }
}

/// A point of the marched segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
    /// Z.
    pub z: f32,
}

/// The world position `0x67ca90` hands the adapter for the cell `(cellx, celly)`, from `0x67cab4`
/// to `0x67cb26`: `x = f32(f32(ox * 33.33333 + cellx * 1.0416666) + 0.5208333)`, and for y the
/// origin term `f32(oy * 33.33333)` is stored (`fstp [ebp-0x14]`) before it is added.
pub fn cell_world_xy(cellx: i32, celly: i32, tile_origin: (i32, i32)) -> (f32, f32) {
    let h = f64::from(STRIDE_CELL) * f64::from(HALF);
    let wx = (f64::from(tile_origin.0) * f64::from(CELL_TO_WORLD)
        + f64::from(cellx) * f64::from(STRIDE_CELL)) as f32;
    let oy = (f64::from(tile_origin.1) * f64::from(CELL_TO_WORLD)) as f32;
    let wy = (f64::from(oy) + f64::from(celly) * f64::from(STRIDE_CELL)) as f32;
    (
        (f64::from(wx) + h) as f32,
        (f64::from(wy) + f64::from(h as f32)) as f32,
    )
}

/// `0x67ca90` GetWeatherCellHeight (`systems/lighting/weather.md#ground-grid`), the whole function:
/// the cell `(cellx, celly)` of the 160 by 160 grid lives in window tile `(cellx >> 5) + 5 *
/// ((celly >> 5) + 2)` at `(celly & 0x1f) * 0x20 + (cellx & 0x1f)`; a FLT_MAX cell is filled with
/// `terrain_height` at [`cell_world_xy`] and the cell is returned.
pub fn cell_index_fill(
    cellx: i32,
    celly: i32,
    tile_origin: (i32, i32),
    tiles: &mut [Tile],
    mut terrain_height: impl FnMut(f32, f32) -> f32,
) -> f32 {
    let win_idx = ((cellx >> 5) + 5 * ((celly >> 5) + 2)) as usize;
    let intra = ((celly & 0x1f) * 0x20 + (cellx & 0x1f)) as usize;
    let (wx, wy) = cell_world_xy(cellx, celly, tile_origin);
    let cells = &mut tiles[win_idx - 10].cells;
    if cells[intra].to_bits() == FLT_MAX_SENTINEL.to_bits() {
        cells[intra] = terrain_height(wx, wy);
    }
    cells[intra]
}

/// `0x67cb60` IntersectWeatherGrid (`systems/lighting/weather.md#ground-grid`), the whole function:
/// the walk from `p0` to `p1` over the 160 by 160 cell grid (`origin_cell` is the grid's `+0x20`
/// and `+0x24`, `tile_origin` its `+0x18` and `+0x1c`). Each endpoint's cell is `floor(0.96 * p) -
/// origin` with the y product stored as f32 first; the driver axis is x when `|dx| <= |dy|` is
/// false (`0x7c13c0`, signed), the step counts `n = max(driver, 1)`, and the error starts at `2 *
/// minor - major` with the steps `2 * minor` and `2 * (minor - major)`; an axis steps backwards
/// when `p0.x > p1.x` or `p0.y > p1.y` (`fcomp; test ah,0x41; jne` keeps the forward step on NaN).
/// The step is `f32(1 / n)` and each pass the point is `p0 + d * t` with `t = i * step` kept wide,
/// `d` stored as f32, and `d.y * t` and `d.z * t` stored as f32 before `p0` is added. A pass
/// outside 0 to 0x9f on a cell axis (checked first, each pass) returns `p0` and false. The cell is
/// sampled ([`cell_index_fill`]) with the point's z; when `z < sample` (`fcomp; test ah,5; jnp`,
/// false on NaN) the point's z becomes the sample and the result is true. Otherwise the error picks
/// the minor step when it is 0 or less and the major step when above 0; at `i == n` the point
/// becomes `p1`; the walk ends false after pass `n`.
pub fn mist_grid_raymarch(
    origin_cell: (i32, i32),
    tile_origin: (i32, i32),
    p0: Vec3,
    p1: Vec3,
    tiles: &mut [Tile],
    mut terrain_height: impl FnMut(f32, f32) -> f32,
) -> (Vec3, bool) {
    let cell = |x: f32, y: f32| -> (i32, i32) {
        let y_scaled = (f64::from(TILE_SCALE_MARCH) * f64::from(y)) as f32;
        (
            ftol_floor(f64::from(TILE_SCALE_MARCH) * f64::from(x)) - origin_cell.0,
            ftol_floor(f64::from(y_scaled)) - origin_cell.1,
        )
    };
    let (c0x, c0y) = cell(p0.x, p0.y);
    let (c1x, c1y) = cell(p1.x, p1.y);
    let dx = (f64::from(p1.x) - f64::from(p0.x)) as f32;
    let dy = (f64::from(p1.y) - f64::from(p0.y)) as f32;
    let dz = (f64::from(p1.z) - f64::from(p0.z)) as f32;
    let adx = (c1x - c0x).abs();
    let ady = (c1y - c0y).abs();
    let steep = adx <= ady;
    let (de_minor, mut err, de_major, n, mut x_minor, mut y_minor) = if steep {
        (adx * 2, adx * 2 - ady, (adx - ady) * 2, ady, 0, 1)
    } else {
        (ady * 2, ady * 2 - adx, (ady - adx) * 2, adx, 1, 0)
    };
    let mut x_major = 1;
    let mut y_major = 1;
    if p0.x > p1.x {
        x_minor = -x_minor;
        x_major = -1;
    }
    if p0.y > p1.y {
        y_minor = -y_minor;
        y_major = -1;
    }
    let n = n.max(1);
    let step = (1.0 / f64::from(n)) as f32;
    let mut cellx = c0x;
    let mut celly = c0y;
    let mut out = p0;
    let mut i = 0i32;
    while (0..0xa0).contains(&cellx) && (0..0xa0).contains(&celly) {
        let t = f64::from(i) * f64::from(step);
        out.x = (f64::from(dx) * t + f64::from(p0.x)) as f32;
        out.y = (f64::from((f64::from(dy) * t) as f32) + f64::from(p0.y)) as f32;
        out.z = (f64::from((f64::from(dz) * t) as f32) + f64::from(p0.z)) as f32;
        let prevz = out.z;
        let sampled = cell_index_fill(cellx, celly, tile_origin, tiles, &mut terrain_height);
        if prevz < sampled {
            out.z = sampled;
            return (out, true);
        }
        let (xinc, yinc, einc) = if err < 1 {
            (x_minor, y_minor, de_minor)
        } else {
            (x_major, y_major, de_major)
        };
        err += einc;
        cellx += xinc;
        celly += yinc;
        if i == n {
            out = p1;
        }
        i += 1;
        if i > n {
            return (out, false);
        }
    }
    (p0, false)
}

/// The identity written to `[ebp-0x68]` before `0x7bdd60`, the tilt matrix's right operand.
const IDENTITY4: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];
/// `[0x807a40]`: the puff size base (the f32 nearest 1/3).
const SPAWN_SIZE_BASE: f32 = f32::from_bits(0x3eaa_aaab);
/// `[0x807334]`: the puff size spread (0.033333335).
const SPAWN_SIZE_SCALE: f32 = f32::from_bits(0x3d08_8889);
/// `[0x80308c]` = 1.5: how far back along its velocity from the camera the puff is placed.
const PLACE_SCALE: f32 = f32::from_bits(0x3fc0_0000);
/// `[0x810034]`: the scale of `+0x128`.
const TAIL_SCALE_128: f32 = f32::from_bits(0x4055_5555);
/// `[0x810030]` = 2.7: the base of the life that `+0x124` quantizes.
const TAIL_BASE_124: f32 = f32::from_bits(0x402c_cccd);
/// `[0x7ffd78]` = 0.3: the spread of that life.
const TAIL_SCALE_124: f32 = f32::from_bits(0x3e99_999a);
/// `[0x7ff9d8]` = 1.0.
const C_1_0: f32 = 1.0;

/// The fields of the effect (`ecx` of `0x67a990`) that the placement reads.
#[derive(Clone, Copy, Debug, Default)]
pub struct MistSpawnSource {
    /// `+0x04`, `+0x08`, `+0x0c`: the extent of the placement box per axis.
    pub pos_extent: [f32; 3],
    /// `+0x1c`: half of it is added to the floored z.
    pub cell_floor_scale: f32,
    /// `+0x24`: the base of the azimuth.
    pub azimuth_base: f32,
    /// `+0x28`: the spread of the azimuth.
    pub azimuth_scale: f32,
    /// `+0x34`: the base of the radius.
    pub radius_base: f32,
    /// `+0x38`: the spread of the radius.
    pub radius_scale: f32,
}

/// What `0x67a990` writes into the puff record.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MistSpawnOut {
    /// `+0x108` to `+0x110`: the position.
    pub world: [f32; 3],
    /// `+0x114` to `+0x11c`: the tilted direction.
    pub dir: [f32; 3],
    /// `+0x120`: the third argument, stored as given.
    pub base_index: u32,
    /// `+0x124`: the third argument plus the quantized life.
    pub tail_index: i32,
    /// `+0x128`: the scale of the sixth draw group.
    pub tail_scale: f32,
}

/// `(m - 1.0) - 0.5` of a draw, the two `fsub`s kept wide.
fn centered(rng: &mut Stream) -> f64 {
    f64::from(rng.unit12()) - f64::from(C_1_0) - 0.5
}

/// `0x67a990` PlacePuff (`systems/lighting/precipitation.md#mist`), the whole function with the
/// callees `0x67ca00` and `0x7bdd60`, `0x7bcae0` run as `grid_sample` and the shared matrix
/// functions. Eight draws, each `(m - 1) - 0.5` wide. The first gives the azimuth `a = d *
/// azimuth_scale + azimuth_base` stored as f32; the second the radius `r` and the third the size `d
/// * 0.033333335 + 0.33333334`, each stored; the direction is `(r * sin a, r * cos a, size)`. The
///   tilt is the axis-angle matrix of `-tilt_angle` (`0x7bdd60`, the guard at `0xc7b530` seeding
///   the axis `(0, 0, 1)` once), and the direction is turned by it (`0x7bcae0`). The next three
///   draws give the placement `(d6 * ext.x, d5 * ext.y, d4 * ext.z)` stored as f32, turned by the
///   same matrix. The position is `cam
/// - dir * 1.5` per axis (the x term stored as f32, the z term from the stored `dir.z * 1.5`) plus
///   the turned placement. The grid height `s` replaces z when `world.z <= s` or NaN (`fcomp; test
///   ah,0x41; jne`), and z becomes `cell_floor_scale * 0.5 + max(z, s)`. The seventh draw gives
///   `+0x128` as `d * 3.3333333` and the eighth `+0x124` as the third argument plus
///   `weather_fixed_quantize(d * 0.3 + 2.7)`.
#[allow(clippy::too_many_arguments)]
pub fn mist_spawn(
    src: &MistSpawnSource,
    base_index: u32,
    tilt_angle: f32,
    tilt_axis: [f32; 3],
    cam: [f32; 3],
    rng: &mut Stream,
    mut grid_sample: impl FnMut([f32; 3]) -> f32,
) -> MistSpawnOut {
    let a = (centered(rng) * f64::from(src.azimuth_scale) + f64::from(src.azimuth_base)) as f32;
    let sa = f64::from(a).sin() as f32;
    let ca = f64::from(a).cos() as f32;
    let r = (centered(rng) * f64::from(src.radius_scale) + f64::from(src.radius_base)) as f32;
    let size = (centered(rng) * f64::from(SPAWN_SIZE_SCALE) + f64::from(SPAWN_SIZE_BASE)) as f32;
    let dir0 = (f64::from(r) * f64::from(sa)) as f32;
    let dir1 = (f64::from(r) * f64::from(ca)) as f32;
    let rot = axis_angle_mat4(&tilt_axis, -tilt_angle, false);
    let tilt4 = mat4_mul(&rot, &IDENTITY4);
    let tilt43: [f32; 12] = tilt4[..12].try_into().unwrap();
    let dir = transform_mat43_vec3(&tilt43, &[dir0, dir1, size]);
    let m4 = centered(rng);
    let m5 = centered(rng);
    let m6 = centered(rng);
    let place = [
        (m6 * f64::from(src.pos_extent[0])) as f32,
        (m5 * f64::from(src.pos_extent[1])) as f32,
        (m4 * f64::from(src.pos_extent[2])) as f32,
    ];
    let world_rot = transform_mat43_vec3(&tilt43, &place);
    let scaled_x = f64::from(dir[0]) * f64::from(PLACE_SCALE);
    let scaled_y = f64::from(dir[1]) * f64::from(PLACE_SCALE);
    let scaled_z = (f64::from(dir[2]) * f64::from(PLACE_SCALE)) as f32;
    let part_x = (f64::from(cam[0]) - scaled_x) as f32;
    let wx = (f64::from(part_x) + f64::from(world_rot[0])) as f32;
    let wy = ((f64::from(cam[1]) - scaled_y) + f64::from(world_rot[1])) as f32;
    let wz = ((f64::from(cam[2]) - f64::from(scaled_z)) + f64::from(world_rot[2])) as f32;
    let mut world = [wx, wy, wz];
    let s = grid_sample(world);
    let floored = if f64::from(s) < f64::from(world[2]) {
        f64::from(world[2])
    } else {
        f64::from(s)
    };
    world[2] = (f64::from(src.cell_floor_scale) * f64::from(HALF) + floored) as f32;
    let tail_scale = (centered(rng) * f64::from(TAIL_SCALE_128)) as f32;
    let q = (centered(rng) * f64::from(TAIL_SCALE_124) + f64::from(TAIL_BASE_124)) as f32;
    let tail_index = weather_fixed_quantize(q).wrapping_add(base_index as i32);
    MistSpawnOut {
        world,
        dir: [dir[0], dir[1], dir[2]],
        base_index,
        tail_index,
        tail_scale,
    }
}
