//! The chunk gathers (`systems/terrain/collide.md#chunk-facets`, `#other-world-queries`):
//! `GatherChunkFacets` (`0x6aadc0`) and `GatherChunkVolumeFacets` (`0x6ac1e0`) add facets to a
//! list, `TestChunkTriangles` (`0x6ad7e0`) and `GatherLiquidCellFacets` (`0x68cec0`) add triangles
//! to the batch at `0xc62550`.
//!
//! Each walks the cells of a rectangle of the chunk, skips holes (bit `(x >> 1) + 4·(y >> 1)` of
//! `0x8103f8` in the hole word at `+0x3c` of the chunk's `+0xf08`), gives the cell's vertices
//! codes against the query, and keeps each of the cell's triangles unless one bit is set in all
//! three of its codes. A box code bit is the sign of the stored `f32((v − bound) + 0.019444443)`.
//! The liquid, doodad, entity and impassable-wall parts of these functions are not modelled; the
//! tests keep their bits clear.

use benilla_re_collision::resolve_toolkit::plane_from_points;

use crate::frustum::{outcode6, Plane};

/// `[0x8101b8]`: 0.019444443 (`0x3c9f49f4`), added to each difference before its store.
const EPS: f32 = f32::from_bits(0x3c9f_49f4);

/// `0x8103f8`: the hole bit of each 2×2 block of cells.
const HOLES_TABLE: [u16; 16] = [
    0x0001, 0x0002, 0x0004, 0x0008, 0x0010, 0x0020, 0x0040, 0x0080, 0x0100, 0x0200, 0x0400, 0x0800,
    0x1000, 0x2000, 0x4000, 0x8000,
];

/// A cell's four triangles from its top-left vertex (the inner vertex at +9, the next row at +17
/// and +18): the three arrays at `0x810438`, `0x81043c` and `0x810440`, 12 bytes apart.
const TRI_FAN: [[usize; 3]; 4] = [[17, 9, 0], [9, 1, 0], [9, 17, 18], [9, 18, 1]];

/// The five vertices given codes, `0x810468`.
const CORNER_FAN: [usize; 5] = [0, 9, 17, 1, 18];

/// A liquid cell's two triangles, `0x810304`, over the 9×9 grid.
const MCLQ_TRI: [[usize; 3]; 2] = [[0, 9, 10], [0, 10, 1]];

/// What the gathers read of a chunk.
pub struct ChunkModel<'a> {
    /// `+0xc`; bit 0x40 adds the impassable walls (`0x6ab530`) first, not modelled here.
    pub flags: u32,
    /// `+0x6c`: the position, added to each vertex a facet stores.
    pub origin: [f32; 3],
    /// `+0x170`: the normals, which a batch header records.
    pub normals_off: u32,
    /// `+0x83c`: the 145 vertices, a cell's top-left at `17·y + x`.
    pub verts: &'a [[f32; 3]],
    /// The hole word.
    pub holes: u16,
}

/// One box bit at `bit`: the sign of `f32((coord − bound) + 0.019444443)`; NaN sets none.
#[inline]
fn oc_bit(coord: f32, bound: f32, bit: u32) -> u32 {
    let r = ((f64::from(coord) - f64::from(bound)) + f64::from(EPS)) as f32;
    (r.to_bits() >> 31) << bit
}

/// A vertex's six box bits against `[min, max]`: x, y, z of `v − min`, then of `max − v`.
#[inline]
fn aabb_outcode(v: &[f32; 3], aabb: &[f32; 6]) -> u32 {
    oc_bit(v[0], aabb[0], 0)
        | oc_bit(v[1], aabb[1], 1)
        | oc_bit(v[2], aabb[2], 2)
        | oc_bit(aabb[3], v[0], 3)
        | oc_bit(aabb[4], v[1], 4)
        | oc_bit(aabb[5], v[2], 5)
}

/// Whether cell `(cx, cy)` is in a hole.
#[inline]
fn cell_holed(cx: usize, cy: usize, holes: u16) -> bool {
    u32::from(HOLES_TABLE[(cx >> 1) + (cy >> 1) * 4]) & u32::from(holes) != 0
}

/// The rectangle clamped to the chunk, in the order of the function's locals `[ebp-0x14]`,
/// `[ebp-0x18]`, `[ebp-0x10]`, `[ebp-0xc]`: the first two at least 0, the others at most 7
/// (`0x6aae6f` to `0x6aae8d`). Rows run from `[1]` to `[2]`, columns from `[0]` to `[3]`.
#[inline]
fn clamp_cell_rect(rect: [i32; 4]) -> [i32; 4] {
    let mut c14 = rect[0];
    let mut c18 = rect[1];
    let mut c10 = rect[2];
    let mut c0c = rect[3];
    if c18 < 0 {
        c18 = 0;
    }
    if c14 < 0 {
        c14 = 0;
    }
    if c10 > 7 {
        c10 = 7;
    }
    if c0c > 7 {
        c0c = 7;
    }
    [c14, c18, c10, c0c]
}

/// A facet of `0x6aadc0` and `0x6ac1e0`: the plane at `+0x0` and the three vertices at `+0x10`,
/// `+0x1c` and `+0x28` of the 0x34-byte record.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlaneRecord {
    pub plane: [f32; 4],
    pub v0: [f32; 3],
    pub v1: [f32; 3],
    pub v2: [f32; 3],
}

/// The facet list: the records, and the array at `+0x18`, 8 bytes an entry, which the terrain
/// grows to the count and zeroes.
#[derive(Default)]
pub struct FacetArray {
    pub records: Vec<PlaneRecord>,
    pub rows: Vec<[f32; 2]>,
}

/// `PlaneFromTriangle` (`0x637480`) of a facet, as `[n, d]`.
#[inline]
fn facet_plane(v0: &[f32; 3], v1: &[f32; 3], v2: &[f32; 3]) -> [f32; 4] {
    let p = plane_from_points(v0, v1, v2);
    [p.n[0], p.n[1], p.n[2], p.d]
}

/// `GatherChunkFacets` (`0x6aadc0`, `systems/terrain/collide.md#chunk-facets`), the terrain (mask
/// 0xf00) without SSE (`0xc9e388` clear): the box taken relative to the chunk, its minimum adding
/// the negated position inline and its maximum through `0x4841c0` (`0x6aaed7`); the rectangle
/// clamped; each kept triangle a facet of the vertices plus the position, its plane from
/// `0x637480`. Then the array at `+0x18` grows to the count, the new entries zero.
pub fn chunk_gather_aabb(
    chunk: &ChunkModel,
    aabb: &[f32; 6],
    cell_rect: [i32; 4],
    out: &mut FacetArray,
    eps_gate: u32,
) {
    debug_assert!(
        chunk.flags & 0x40 == 0,
        "an impassable chunk's walls are not modelled here"
    );
    debug_assert!(
        eps_gate == 0,
        "the rsqrtss path (0xc9e388 set) differs by CPU and is not modelled"
    );
    let nb = [
        (f64::from(aabb[0]) + f64::from(-chunk.origin[0])) as f32,
        (f64::from(aabb[1]) + f64::from(-chunk.origin[1])) as f32,
        (f64::from(aabb[2]) + f64::from(-chunk.origin[2])) as f32,
        (f64::from(aabb[3]) + f64::from(-chunk.origin[0])) as f32,
        (f64::from(aabb[4]) + f64::from(-chunk.origin[1])) as f32,
        (f64::from(aabb[5]) + f64::from(-chunk.origin[2])) as f32,
    ];
    let [c14, c18, c10, c0c] = clamp_cell_rect(cell_rect);
    let mut oc = [0u32; 19];
    for row in c18..=c10 {
        for col in c14..=c0c {
            let cx = col as usize & 7;
            let cy = row as usize & 7;
            if cell_holed(cx, cy, chunk.holes) {
                continue;
            }
            let base = cy * 0x11 + cx;
            for &k in CORNER_FAN.iter() {
                oc[k] = aabb_outcode(&chunk.verts[base + k], &nb);
            }
            for tri in TRI_FAN.iter() {
                if oc[tri[0]] & oc[tri[1]] & oc[tri[2]] != 0 {
                    continue;
                }
                let v0 = world_vert(chunk, base + tri[0]);
                let v1 = world_vert(chunk, base + tri[1]);
                let v2 = world_vert(chunk, base + tri[2]);
                out.records.push(PlaneRecord {
                    plane: facet_plane(&v0, &v1, &v2),
                    v0,
                    v1,
                    v2,
                });
            }
        }
    }
    out.rows.resize(out.records.len(), [0.0, 0.0]);
}

/// `GatherChunkVolumeFacets` (`0x6ac1e0`, `systems/terrain/collide.md#other-world-queries`): the
/// same walk with each vertex's code from `GetPlaneOutcode` (`0x686c20`) against six planes the
/// caller made relative to the chunk (`0x686820`). The chunk's doodads (`+0xe4` through
/// `0x7137c0`) are not modelled. True when a facet was added.
pub fn chunk_gather_planes(
    chunk: &ChunkModel,
    planes: &[Plane; 6],
    cell_rect: [i32; 4],
    out: &mut FacetArray,
) -> bool {
    let start = out.records.len();
    let [c14, c18, c10, c0c] = clamp_cell_rect(cell_rect);
    let mut oc = [0u32; 19];
    for row in c18..=c10 {
        for col in c14..=c0c {
            let cx = col as usize & 7;
            let cy = row as usize & 7;
            if cell_holed(cx, cy, chunk.holes) {
                continue;
            }
            let base = cy * 0x11 + cx;
            for &k in CORNER_FAN.iter() {
                oc[k] = outcode6(planes, &chunk.verts[base + k]);
            }
            for tri in TRI_FAN.iter() {
                if oc[tri[0]] & oc[tri[1]] & oc[tri[2]] != 0 {
                    continue;
                }
                let v0 = world_vert(chunk, base + tri[0]);
                let v1 = world_vert(chunk, base + tri[1]);
                let v2 = world_vert(chunk, base + tri[2]);
                out.records.push(PlaneRecord {
                    plane: facet_plane(&v0, &v1, &v2),
                    v0,
                    v1,
                    v2,
                });
            }
        }
    }
    out.records.len() != start
}

/// A vertex plus the chunk's position, each component stored as `f32`.
#[inline]
fn world_vert(chunk: &ChunkModel, idx: usize) -> [f32; 3] {
    let v = &chunk.verts[idx];
    [
        (f64::from(v[0]) + f64::from(chunk.origin[0])) as f32,
        (f64::from(v[1]) + f64::from(chunk.origin[1])) as f32,
        (f64::from(v[2]) + f64::from(chunk.origin[2])) as f32,
    ]
}

/// A batch header, 0x20 bytes at `0xc62550 + 0x20·[0xc5a474]`: the matrix at `+0x0`, the vertices
/// at `+0x4`, the normals at `+0x8`, the indices at `+0xc`, the index and triangle counts (`u16`)
/// at `+0x14` and `+0x16`, the least and greatest index at `+0x18` and `+0x1a`, the owner at
/// `+0x1c`. `index_base` is the cursor `0xc7b378` when the batch opened; the whole span the
/// rectangle could need is reserved then, and only `index_count` indices are written.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BatchHeader {
    pub verts_base: u32,
    pub normals_base: u32,
    pub owner: u32,
    pub index_base: u32,
    pub index_count: u16,
    pub tri_count: u16,
    pub index_min: u16,
    pub index_max: u16,
}

/// The batch: the headers (at most 0x20), the matrices at `0xc62980` (count `0xc63190`, at most
/// 0x20), the `u16` indices at `0xc63298` up to 0xc000 with the cursor `0xc7b378`, and the
/// overflow flag `0xc63258`.
#[derive(Default)]
pub struct BatchProtocol {
    pub headers: Vec<BatchHeader>,
    pub matrix_count: u32,
    pub index_cursor: u32,
    /// Each written index with its slot.
    indices: Vec<(u32, u16)>,
    pub overflow: bool,
}

impl BatchProtocol {
    /// The indices written for header `h`, from `index_base` for `index_count`.
    pub fn written_indices(&self, h: &BatchHeader) -> Vec<u16> {
        let lo = h.index_base;
        let hi = lo + u32::from(h.index_count);
        self.indices
            .iter()
            .filter(|(slot, _)| *slot >= lo && *slot < hi)
            .map(|(_, v)| *v)
            .collect()
    }
}

const INDEX_CAP: u32 = 0xc000;
const SLOT_CAP: u32 = 0x20;

impl BatchProtocol {
    /// Opens a batch at the first kept triangle. A header count reaching 0x20 sets the overflow and
    /// opens nothing; otherwise the header is taken, and a matrix count reaching 0x20 or `cursor +
    /// span` reaching 0xc000 sets the overflow and leaves the batch without matrix or indices, so
    /// nothing is written to it. The cursor moves by the span when it fits.
    fn open(&mut self, verts_base: u32, normals_base: u32, owner: u32, span: u32) -> Option<usize> {
        if self.headers.len() as u32 + 1 >= SLOT_CAP {
            self.overflow = true;
            return None;
        }
        let matrix_ok = self.matrix_count + 1 < SLOT_CAP;
        let index_base = self.index_cursor;
        let index_ok = self.index_cursor + span < INDEX_CAP;
        if !matrix_ok || !index_ok {
            self.overflow = true;
        }
        if index_ok {
            self.index_cursor += span;
        }
        self.headers.push(BatchHeader {
            verts_base,
            normals_base,
            owner,
            index_base,
            index_count: 0,
            tri_count: 0,
            index_min: 0xffff,
            index_max: 0,
        });
        if matrix_ok {
            self.matrix_count += 1;
        }
        if matrix_ok && index_ok {
            Some(self.headers.len() - 1)
        } else {
            None
        }
    }

    /// Writes a triangle's three indices after the header's last, and updates the counts and the
    /// least (`<=`) and greatest (`>`) index.
    fn emit_tri(&mut self, hdr: usize, idx: [u16; 3]) {
        let h = &mut self.headers[hdr];
        let mut min = h.index_min;
        let mut max = h.index_max;
        let base_slot = h.index_base + u32::from(h.index_count);
        for (k, &i) in idx.iter().enumerate() {
            self.indices.push((base_slot + k as u32, i));
            if i <= min {
                min = i;
            }
            if i > max {
                max = i;
            }
        }
        h.index_count = h.index_count.wrapping_add(3);
        h.tri_count = h.tri_count.wrapping_add(1);
        h.index_min = min;
        h.index_max = max;
    }
}

/// `TestChunkTriangles` (`0x6ad7e0`, `systems/terrain/collide.md#other-world-queries`): the box
/// as given, not relative to the chunk, and the rectangle not clamped (the caller clamps); the kept
/// triangles' indices `17·row + column + k` go into a batch the first opens, with the vertices
/// `verts_base`, the normals `+0x170` and a span of `(y1 + 1 − y0)·(x1 − x0 + 1)·12`. The hole bit
/// is that of `(column, row)`. True when a triangle was kept.
pub fn chunk_collect_aabb(
    chunk: &ChunkModel,
    aabb: &[f32; 6],
    cell_rect: [i32; 4],
    verts_base: u32,
    owner: u32,
    batch: &mut BatchProtocol,
) -> bool {
    let span = ((cell_rect[3] + 1 - cell_rect[1]).max(0)
        * (cell_rect[2] - cell_rect[0] + 1).max(0)
        * 12) as u32;
    let mut oc = [0u32; 19];
    let mut hdr: Option<usize> = None;
    let mut emitted = false;
    for ox in cell_rect[0]..=cell_rect[2] {
        for iy in cell_rect[1]..=cell_rect[3] {
            let cx = iy as usize & 7;
            let cy = ox as usize & 7;
            if cell_holed(cx, cy, chunk.holes) {
                continue;
            }
            let base = (ox as usize) * 0x11 + iy as usize;
            for &k in CORNER_FAN.iter() {
                oc[k] = aabb_outcode(&chunk.verts[base + k], aabb);
            }
            let window_base = base as u16;
            for tri in TRI_FAN.iter() {
                if oc[tri[0]] & oc[tri[1]] & oc[tri[2]] != 0 {
                    continue;
                }
                if !emitted {
                    hdr = batch.open(verts_base, chunk.normals_off, owner, span);
                    emitted = true;
                }
                if let Some(h) = hdr {
                    batch.emit_tri(
                        h,
                        [
                            window_base + tri[0] as u16,
                            window_base + tri[1] as u16,
                            window_base + tri[2] as u16,
                        ],
                    );
                }
            }
        }
    }
    emitted
}

/// `GatherLiquidCellFacets` (`0x68cec0`): the liquid's 81 vertices coded on z alone, bit 2 the sign
/// of `f32((z − min z) + ε)` (`0x68cf3f`) and bit 5 that of `f32((max z − z) + ε)` (`0x68cf5d`); for
/// each cell of `[x0, y0, x1, y1]` whose type nibble (`flags[8x + y]`) is not 0xf, the two triangles
/// of `0x810304` from vertex `9x + y`, kept as above, go into a batch with the vertices
/// `layer_verts_base` (the liquid's `+0x34`), no normals and a span of `(y1 + 1 − y0)·6·(x1 − x0 +
/// 1)`. True when a triangle was kept.
pub fn mclq_collect(
    layer_verts_base: u32,
    owner: u32,
    query: &[f32; 6],
    flags: &[u8; 64],
    verts_z: &[f32; 81],
    rect: [i32; 4],
    batch: &mut BatchProtocol,
) -> bool {
    let mut oc = [0u32; 81];
    for (i, &z) in verts_z.iter().enumerate() {
        let lo = ((f64::from(z) - f64::from(query[2])) + f64::from(EPS)) as f32;
        let hi = ((f64::from(query[5]) - f64::from(z)) + f64::from(EPS)) as f32;
        oc[i] = ((lo.to_bits() >> 31) << 2) | ((hi.to_bits() >> 31) << 5);
    }

    let mut hdr: Option<usize> = None;
    let mut emitted = false;
    let span = ((rect[3] + 1 - rect[1]).max(0) * 6 * (rect[2] - rect[0] + 1).max(0)) as u32;

    for x in rect[0]..=rect[2] {
        for y in rect[1]..=rect[3] {
            let cx = x as usize;
            let cy = y as usize;
            if flags[cx * 8 + cy] & 0xf == 0xf {
                continue;
            }
            let window_base = (cx * 9 + cy) as u16;
            let wb = window_base as usize;
            for tri in MCLQ_TRI.iter() {
                let a = wb + tri[0];
                let b = wb + tri[1];
                let c = wb + tri[2];
                if oc[a] & oc[b] & oc[c] != 0 {
                    continue;
                }
                if !emitted {
                    hdr = batch.open(layer_verts_base, 0, owner, span);
                    emitted = true;
                }
                if let Some(h) = hdr {
                    batch.emit_tri(
                        h,
                        [
                            window_base + tri[0] as u16,
                            window_base + tri[1] as u16,
                            window_base + tri[2] as u16,
                        ],
                    );
                }
            }
        }
    }
    emitted
}
