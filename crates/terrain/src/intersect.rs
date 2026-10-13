//! The tests of the listed cells: `TraceListedCells` (`0x69c920`) for `TraceTerrainSegment` and
//! `TraceListedCellsFirstHit` (`0x6aa600`) for `TraceTerrainFirstHit`
//! (`systems/terrain/trace.md#cell-walks`, `#terrain-first-hit`).
//!
//! Each cell of the list (`crate::dda::CellList`) is resolved through the area table: the tile from
//! `>> 7 & 0x3f` of both integers, the chunk from `>> 3 & 0xf`, the cell from `& 7`, the second
//! integer the major index. An integer above 0x2000 ends the list. A cell in a hole is skipped; the
//! rest test the four triangles around the cell's inner vertex with `IntersectRayTriangle`
//! (`0x7c2c40`).

#![allow(clippy::neg_cmp_op_on_partial_ord)] // a negated compare is how a NaN takes the client's branch

use benilla_re_cmath::intersect::{segment_tri_moller_liquid, Ray};
use benilla_re_collision::resolve_toolkit::plane_from_points;

use crate::dda::{CellIntersector, CellList, Ray6};

/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);

/// `[0x7ffd74]`: 0.0.
const ZERO: f32 = f32::from_bits(0x0000_0000);

/// 0.01 (`0x3c23d70a`), the slack `0x69c920` pushes for `0x7c2c40`.
const RAYTRI_EPS: f32 = f32::from_bits(0x3c23_d70a);

/// `[0x8029d0]`: 0.01, the least segment length `0x6aa600` tests.
const MIN_LEN: f32 = f32::from_bits(0x3c23_d70a);

/// `0xc96318`: the area table.
pub const AREA_TABLE_VA: u32 = 0x00c9_6318;

/// The world as the cell tests read it.
pub trait RaycastWorld {
    /// The tile pointer at `0xc96318 + 4·index`, 0 when absent.
    fn tile_ptr(&self, grid_index: usize) -> u32;
    /// The tile's `+0x278 + 4·index`, the chunk pointer, 0 when absent.
    fn chunk_ptr(&self, tile: u32, chunk_index: usize) -> u32;
    /// The `u16` at `+0x3c` of the chunk's `+0xf08`: the hole bits.
    fn hole_mask(&self, chunk: u32) -> u16;
    /// Vertex `index` of the 145 at the chunk's `+0x83c`.
    fn terrain_vert(&self, chunk: u32, index: i32) -> [f32; 3];
    /// The chunk's `+0x118 + 4·layer`, a liquid, 0 when absent.
    fn liquid_layer(&self, chunk: u32, layer: usize) -> u32;
    /// The low nibble of byte `y·8 + x` at the liquid's `+0x10`, the cell's type.
    fn liquid_type_nibble(&self, layer: u32, x: i32, y: i32) -> u8;
    /// Vertex `index` of the 81 at the liquid's `+0x34`.
    fn liquid_vert(&self, layer: u32, index: i32) -> [f32; 3];
}

/// `IntersectRayTriangle` (`0x7c2c40`) on three vertices with slack `eps`: `t` along `dir` on a hit.
fn ray_tri(
    origin: &[f32; 3],
    dir: &[f32; 3],
    v0: &[f32; 3],
    v1: &[f32; 3],
    v2: &[f32; 3],
    eps: f32,
) -> Option<f32> {
    let ray = Ray {
        origin: *origin,
        dir: *dir,
    };
    let mut t = 0.0f32;
    segment_tri_moller_liquid(&ray, &[*v0, *v1, *v2], &[0, 1, 2], Some(&mut t), None, eps)
        .then_some(t)
}

/// A resolved cell: its chunk, the index of its top-left vertex, its cell coordinates and the
/// chunk bits the next cell is compared with.
struct ResolvedCell {
    chunk: u32,
    /// `(c1 & 7)·17 + (c0 & 7)`.
    idx: i32,
    /// `c1 & 7`.
    qx: i32,
    /// `c0 & 7`.
    qy: i32,
    /// `c0 & 0x1ff8` and `c1 & 0x1ff8`.
    bits0: u32,
    bits1: u32,
}

/// A list entry `(c0, c1)` resolved: the tile `(c1 >> 7 & 0x3f)·64 + (c0 >> 7 & 0x3f)`, the chunk
/// `(c1 >> 3 & 0xf)·16 + (c0 >> 3 & 0xf)`; a cell in the same chunk as the last (`& 0x1ff8` of
/// both equal) reuses it. `None`, which ends the walk, when the tile or chunk is absent.
fn resolve_cell(
    world: &dyn RaycastWorld,
    c0: u32,
    c1: u32,
    prev_chunk: u32,
    cached: (u32, u32),
) -> Option<ResolvedCell> {
    let bits0 = c0 & 0x1ff8;
    let bits1 = c1 & 0x1ff8;
    let qx = (c1 & 7) as i32;
    let qy = (c0 & 7) as i32;
    let idx = ((c1 & 7) * 0x11 + (c0 & 7)) as i32;

    if bits0 == cached.0 && bits1 == cached.1 {
        return Some(ResolvedCell {
            chunk: prev_chunk,
            idx,
            qx,
            qy,
            bits0,
            bits1,
        });
    }

    let grid_index = (((c1 >> 7) & 0x3f) * 0x40 + ((c0 >> 7) & 0x3f)) as usize;
    let tile = world.tile_ptr(grid_index);
    if tile == 0 {
        return None;
    }
    let chunk_index = (((c1 >> 3) & 0xf) * 0x10 + ((c0 >> 3) & 0xf)) as usize;
    let chunk = world.chunk_ptr(tile, chunk_index);
    if chunk == 0 {
        return None;
    }
    Some(ResolvedCell {
        chunk,
        idx,
        qx,
        qy,
        bits0,
        bits1,
    })
}

/// `0x8103f8`: the hole bit of each 2×2 block of cells.
const HOLE_BIT_TABLE: [u32; 16] = [
    0x0000_0001,
    0x0000_0002,
    0x0000_0004,
    0x0000_0008,
    0x0000_0010,
    0x0000_0020,
    0x0000_0040,
    0x0000_0080,
    0x0000_0100,
    0x0000_0200,
    0x0000_0400,
    0x0000_0800,
    0x0000_1000,
    0x0000_2000,
    0x0000_4000,
    0x0000_8000,
];

/// Whether the cell is in a hole: bit `(c0q >> 1) + 4·(c1q >> 1)` of the table against the mask.
fn is_hole(hole_mask: u16, c0q: i32, c1q: i32) -> bool {
    let idx = ((c0q >> 1) + (c1q >> 1) * 4) as usize;
    (HOLE_BIT_TABLE[idx] & u32::from(hole_mask)) != 0
}

/// The four triangles of `0x69c920`, from the table at `0x81047c`: the inner vertex `idx + 9`, then
/// `idx + a` and `idx + b`.
const FAN_CMAP: [(i32, i32); 4] = [(0, 17), (1, 0), (17, 18), (18, 1)];
/// Those of `0x6aa600`, from `0x810480`: the same vertices with the other two in reverse, so each
/// triangle faces the other way.
const FAN_FIRST: [(i32, i32); 4] = [(17, 0), (0, 1), (18, 17), (1, 18)];

/// What `0x69c920` returns and writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellHit {
    /// `eax`: a hit nearer than the fraction given.
    pub hit: bool,
    /// The fraction afterwards, unchanged without a hit.
    pub t: f32,
    /// The chunk hit, written only on a hit.
    pub chunk: u32,
}

/// `TraceListedCells` (`0x69c920`): the nearest hit of the segment `a`–`b` over the listed cells,
/// for the mask bits 0x100 (terrain) and 0x10000 << layer (liquid layers). The direction is
/// `(b − a)·f32(1/√((dz·f32(dz) + dy²) + dx²))`, each component stored; a triangle's `t` is scaled
/// by the same factor and kept when `t < best` and `t >= 0`. A liquid cell whose type is not 0xf
/// tests two triangles of the 9×9 grid. The fraction is written when the best is below the one
/// given. The mask bits for doodads and map objects call other code and are not modelled.
pub fn dda_cell_intersect(
    world: &dyn RaycastWorld,
    list: &CellList,
    a: &[f32; 3],
    b: &[f32; 3],
    t_in: f32,
    mask: u32,
) -> CellHit {
    // dx and dy stay in the register; dz is also stored (`fst [ebp-0x24]`), and its square takes
    // one of each.
    let dx = f64::from(b[0]) - f64::from(a[0]);
    let dy = f64::from(b[1]) - f64::from(a[1]);
    let dz = f64::from(b[2]) - f64::from(a[2]);
    let dz_f32 = dz as f32;
    let len2 = (dz * f64::from(dz_f32) + dy * dy) + dx * dx;
    let inv_len = (f64::from(ONE) / len2.sqrt()) as f32; // `[ebp-0x10]`
    let dir = [
        (dx * f64::from(inv_len)) as f32,
        (dy * f64::from(inv_len)) as f32,
        (f64::from(dz_f32) * f64::from(inv_len)) as f32,
    ];

    let terrain_on = (mask & 0x100) != 0;

    let mut best_t = t_in;
    let mut best_chunk: u32 = 0;
    let mut prev_chunk: u32 = 0;
    // The cache starts from the list head's 0x2000 bit, outside the 0x1ff8 mask, so the first cell
    // never matches.
    let mut cache: (u32, u32) = (0x2000, 0x2000);

    let pairs = list.pairs();
    for &(c0_i, c1_i) in pairs.iter() {
        let c0 = c0_i as u32;
        let c1 = c1_i as u32;
        if c0 > 0x2000 || c1 > 0x2000 {
            break;
        }
        let Some(cell) = resolve_cell(world, c0, c1, prev_chunk, cache) else {
            break;
        };
        prev_chunk = cell.chunk;
        cache = (cell.bits0, cell.bits1);

        let hole_mask = world.hole_mask(cell.chunk);

        if terrain_on && !is_hole(hole_mask, cell.qy, cell.qx) {
            let idx = cell.idx;
            let center = world.terrain_vert(cell.chunk, idx + 9);
            for &(v1off, v2off) in FAN_CMAP.iter() {
                let v1 = world.terrain_vert(cell.chunk, v1off + idx);
                let v2 = world.terrain_vert(cell.chunk, v2off + idx);
                if let Some(ray_t) = ray_tri(a, &dir, &center, &v1, &v2, RAYTRI_EPS) {
                    let t = (f64::from(ray_t) * f64::from(inv_len)) as f32;
                    if accept(t, best_t) {
                        best_chunk = cell.chunk;
                        best_t = t;
                    }
                }
            }
        }

        if (mask & 0xf0000) != 0 {
            for layer in 0..4usize {
                let layer_bit = 0x10000u32 << layer;
                let lobj = world.liquid_layer(cell.chunk, layer);
                // The type byte is at `c1q·8 + c0q`.
                if (layer_bit & mask) != 0
                    && lobj != 0
                    && world.liquid_type_nibble(lobj, cell.qy, cell.qx) != 0xf
                {
                    let lidx = cell.qy + cell.qx * 9;
                    for &(o0, o1, o2) in LIQUID_TRIS.iter() {
                        let lv0 = world.liquid_vert(lobj, o0 + lidx);
                        let lv1 = world.liquid_vert(lobj, o1 + lidx);
                        let lv2 = world.liquid_vert(lobj, o2 + lidx);
                        if let Some(ray_t) = ray_tri(a, &dir, &lv0, &lv1, &lv2, RAYTRI_EPS) {
                            let t = (f64::from(ray_t) * f64::from(inv_len)) as f32;
                            if accept(t, best_t) {
                                best_chunk = cell.chunk;
                                best_t = t;
                            }
                        }
                    }
                }
            }
        }
    }

    // `fld best; fcomp [t]`: only a strict improvement is written; NaN is not.
    if f64::from(best_t) < f64::from(t_in) {
        CellHit {
            hit: true,
            t: best_t,
            chunk: best_chunk,
        }
    } else {
        CellHit {
            hit: false,
            t: t_in,
            chunk: 0,
        }
    }
}

/// The liquid cell's two triangles, from `0x81049c`: `(0, 10, 9)` and `(0, 1, 10)` from the cell's
/// top-left vertex.
const LIQUID_TRIS: [(i32, i32, i32); 2] = [(0, 10, 9), (0, 1, 10)];

/// A hit is kept when `t < best` (`fcom; test ah,5; jp`) and `t >= 0` (`test ah,1; jne`); NaN is
/// not.
fn accept(t: f32, best: f32) -> bool {
    f64::from(t) < f64::from(best) && f64::from(t) >= f64::from(ZERO)
}

/// What `0x6aa600` returns and writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstHit {
    /// `al`: a triangle was hit.
    pub hit: bool,
    /// The fraction afterwards, unchanged without a hit.
    pub t: f32,
    /// The hit triangle's plane from `0x637480`, written only on a hit.
    pub plane: Option<[f32; 4]>,
}

/// `TraceListedCellsFirstHit` (`0x6aa600`): the terrain alone, slack 0, the triangles of
/// `FAN_FIRST`. A segment shorter than 0.01, or NaN, returns no hit at once. The direction is
/// `(b − a)/√((dx² + f32(dy)²) + dz²)`, the reciprocal kept in the register. A hit, `t < best` and
/// `t >= 0` with `t` along the direction unscaled, records the triangle's plane through
/// `PlaneFromTriangle` (`0x637480`); the walk stops after the cell with the first hit.
pub fn dda_cell_intersect_first(
    world: &dyn RaycastWorld,
    list: &CellList,
    seg: &[f32; 6],
    t_io: f32,
) -> FirstHit {
    let a = [seg[0], seg[1], seg[2]];
    let b = [seg[3], seg[4], seg[5]];

    // dx and dz stay in the register; dy is stored (`[ebp-0x2c]`).
    let dx = f64::from(b[0]) - f64::from(a[0]);
    let dy_f32 = (f64::from(b[1]) - f64::from(a[1])) as f32;
    let dz = f64::from(b[2]) - f64::from(a[2]);
    let len2 = (dx * dx + f64::from(dy_f32) * f64::from(dy_f32)) + dz * dz;
    let len = (len2.sqrt()) as f32;
    if !(f64::from(len) >= f64::from(MIN_LEN)) {
        return FirstHit {
            hit: false,
            t: t_io,
            plane: None,
        };
    }
    let inv_len = f64::from(ONE) / len2.sqrt();
    let dir = [
        (inv_len * dx) as f32,
        (inv_len * f64::from(dy_f32)) as f32,
        (inv_len * dz) as f32,
    ];

    let mut best_t = t_io;
    let mut plane: Option<[f32; 4]> = None;
    let mut hit = false;
    let mut prev_chunk: u32 = 0;
    let mut cache: (u32, u32) = (0x2000, 0x2000);

    let pairs = list.pairs();
    'walk: for &(c0_i, c1_i) in pairs.iter() {
        let c0 = c0_i as u32;
        let c1 = c1_i as u32;
        if c0 > 0x2000 || c1 > 0x2000 {
            break;
        }
        let Some(cell) = resolve_cell(world, c0, c1, prev_chunk, cache) else {
            break;
        };
        prev_chunk = cell.chunk;
        cache = (cell.bits0, cell.bits1);

        let hole_mask = world.hole_mask(cell.chunk);
        // A hole skips to the next cell.
        if is_hole(hole_mask, cell.qy, cell.qx) {
            continue;
        }

        let idx = cell.idx;
        let center = world.terrain_vert(cell.chunk, idx + 9);
        for &(v1off, v2off) in FAN_FIRST.iter() {
            let v1 = world.terrain_vert(cell.chunk, v1off + idx);
            let v2 = world.terrain_vert(cell.chunk, v2off + idx);
            if let Some(ray_t) = ray_tri(&a, &dir, &center, &v1, &v2, 0.0) {
                if accept(ray_t, best_t) {
                    best_t = ray_t;
                    hit = true;
                    let p = plane_from_points(&center, &v1, &v2);
                    plane = Some([p.n[0], p.n[1], p.n[2], p.d]);
                }
            }
        }
        if hit {
            break 'walk;
        }
    }

    FirstHit {
        hit,
        t: best_t,
        plane,
    }
}

/// `0x69c920` behind `ray_dda_map`, with the mask and the chunk hit.
pub struct MapIntersector<'w> {
    pub world: &'w dyn RaycastWorld,
    /// The mask: terrain 0x100, liquid 0x10000 << layer.
    pub mask: u32,
    /// The chunk hit.
    pub out_chunk: u32,
    /// Whether a call hit.
    pub hit: bool,
}

impl CellIntersector for MapIntersector<'_> {
    fn intersect(&mut self, list: &CellList, ray: &Ray6, t_io: &mut f32) -> bool {
        let r = dda_cell_intersect(
            self.world,
            list,
            &ray.origin,
            &ray.endpoint,
            *t_io,
            self.mask,
        );
        if r.hit {
            *t_io = r.t;
            self.out_chunk = r.chunk;
            self.hit = true;
        }
        r.hit
    }
}

/// `0x6aa600` behind `ray_dda_chunk`, which scales the fraction it writes; with the plane hit.
pub struct FirstIntersector<'w> {
    pub world: &'w dyn RaycastWorld,
    /// The hit triangle's plane.
    pub plane: Option<[f32; 4]>,
}

impl CellIntersector for FirstIntersector<'_> {
    fn intersect(&mut self, list: &CellList, ray: &Ray6, t_io: &mut f32) -> bool {
        let seg = [
            ray.origin[0],
            ray.origin[1],
            ray.origin[2],
            ray.endpoint[0],
            ray.endpoint[1],
            ray.endpoint[2],
        ];
        let r = dda_cell_intersect_first(self.world, list, &seg, *t_io);
        if r.hit {
            *t_io = r.t;
            self.plane = r.plane;
        }
        r.hit
    }
}
