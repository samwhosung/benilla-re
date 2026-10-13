//! Three functions that add facets to a collision list: a quad, the walls of an impassable chunk
//! and the liquid triangles (`systems/terrain/collide.md`).
//!
//! A facet is 0x34 bytes, thirteen floats: the plane's normal and `d = −n·v0`, then three vertices
//! (the `imul …,0x34` of the list growers `0x6742d0`, `0x674130` and `0x6aca50`). The growers only
//! make room and return the next slot, so a push models them.

use crate::leaves::plane_from_np;
use crate::query::{vec3_cross_world, vec3_normalize_world};
use benilla_re_cmath::transform::transform_point;
use benilla_re_objectlayer::spline_anchor::dot3;

/// A facet: the normal at `[0..3]`, `d` at `[3]`, the vertices at `[4..7]`, `[7..10]` and
/// `[10..13]`.
pub type Facet = [f32; 13];

/// 32000.0 (`0x46fa0000`), the height a wall rises above the chunk's lowest point.
const WALL_TOP_Z: f32 = f32::from_bits(0x46fa_0000);

/// `AddQuadFacets` (`0x6aba30`): the quad at `p` with edges `e0` and `e1` as two facets on the
/// plane `(n, −(n.y·p.y + n.x·p.x + n.z·p.z))`, the normal as given. The first is `p`, `p + e1`,
/// `p + e1 + e0` with the far corner summed in the register and stored once; the second is `p`, the
/// far corner as `f32(f32(p + e1) + e0)` through two calls of `0x4549a0`, and `p + e0`.
pub fn quad_collision_emit(
    list: &mut Vec<Facet>,
    p: &[f32; 3],
    n: &[f32; 3],
    e0: &[f32; 3],
    e1: &[f32; 3],
) {
    // `0x6aba89`
    let d = (-(f64::from(n[1]) * f64::from(p[1])
        + f64::from(n[0]) * f64::from(p[0])
        + f64::from(n[2]) * f64::from(p[2]))) as f32;

    let p_e1 = [
        (f64::from(p[0]) + f64::from(e1[0])) as f32,
        (f64::from(p[1]) + f64::from(e1[1])) as f32,
        (f64::from(p[2]) + f64::from(e1[2])) as f32,
    ];

    // `0x6abaed`: `fld p; fadd e1; fadd e0; fstp`.
    let far_single = [
        (f64::from(p[0]) + f64::from(e1[0]) + f64::from(e0[0])) as f32,
        (f64::from(p[1]) + f64::from(e1[1]) + f64::from(e0[1])) as f32,
        (f64::from(p[2]) + f64::from(e1[2]) + f64::from(e0[2])) as f32,
    ];
    list.push(make_facet(*n, d, *p, p_e1, far_single));

    let far_double = [
        (f64::from(p_e1[0]) + f64::from(e0[0])) as f32,
        (f64::from(p_e1[1]) + f64::from(e0[1])) as f32,
        (f64::from(p_e1[2]) + f64::from(e0[2])) as f32,
    ];
    // `0x6abc04`
    let p_e0 = [
        (f64::from(p[0]) + f64::from(e0[0])) as f32,
        (f64::from(p[1]) + f64::from(e0[1])) as f32,
        (f64::from(p[2]) + f64::from(e0[2])) as f32,
    ];
    list.push(make_facet(*n, d, *p, far_double, p_e0));
}

/// A facet from its plane and vertices.
#[inline]
fn make_facet(n: [f32; 3], d: f32, v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> Facet {
    [
        n[0], n[1], n[2], d, v0[0], v0[1], v0[2], v1[0], v1[1], v1[2], v2[0], v2[1], v2[2],
    ]
}

/// The chunk's box as `0x6ab530` reads it: `+0x44`, `+0x48`, `+0x4c` (the lowest z), `+0x50` and
/// `+0x54`.
#[derive(Clone, Copy, Debug)]
pub struct ChunkBounds {
    /// `+0x44`
    pub min_x: f32,
    /// `+0x48`
    pub min_y: f32,
    /// `+0x4c`
    pub z: f32,
    /// `+0x50`
    pub max_x: f32,
    /// `+0x54`
    pub max_y: f32,
}

/// `CMapChunk::AddImpassableWalls` (`0x6ab530`, `systems/terrain/collide.md#impassable-walls`): a
/// wall of two facets on each side of the chunk's box that the query box `p` (min then max) crosses,
/// from the chunk's lowest z up 32000. The sides in order: `p.min_y < min_y` with normal (0, −1, 0),
/// `max_y < p.max_y` with (0, 1, 0) and `d` from `0x602630`, `p.min_x < min_x` with (−1, 0, 0), and
/// `max_x < p.max_x` through `AddQuadFacets` with (1, 0, 0); the first and third planes come from
/// `SetPlaneFromNormal` (`0x6acde0`). Every normal points out of the chunk.
pub fn impassable_chunk_fence(list: &mut Vec<Facet>, bounds: &ChunkBounds, p: &[f32; 6]) {
    if p[1] < bounds.min_y {
        let base = [bounds.min_x, bounds.min_y, bounds.z];
        let horiz = [
            (f64::from(bounds.max_x) - f64::from(bounds.min_x)) as f32,
            0.0,
            0.0,
        ];
        let vert = [0.0, 0.0, WALL_TOP_Z];
        emit_wall(list, &[0.0, -1.0, 0.0], &base, &horiz, &vert, false);
    }

    if bounds.max_y < p[4] {
        let base = [bounds.max_x, bounds.max_y, bounds.z];
        let horiz = [
            (f64::from(bounds.min_x) - f64::from(bounds.max_x)) as f32,
            0.0,
            0.0,
        ];
        let vert = [0.0, 0.0, WALL_TOP_Z];
        emit_wall(list, &[0.0, 1.0, 0.0], &base, &horiz, &vert, true);
    }

    if p[0] < bounds.min_x {
        let base = [bounds.min_x, bounds.max_y, bounds.z];
        let horiz = [
            0.0,
            (f64::from(bounds.min_y) - f64::from(bounds.max_y)) as f32,
            0.0,
        ];
        let vert = [0.0, 0.0, WALL_TOP_Z];
        emit_wall(list, &[-1.0, 0.0, 0.0], &base, &horiz, &vert, false);
    }

    if bounds.max_x < p[3] {
        // `0x6aba13`: e0 the rise, e1 the side.
        let base = [bounds.max_x, bounds.min_y, bounds.z];
        let e0_vert = [0.0, 0.0, WALL_TOP_Z];
        let e1_horiz = [
            0.0,
            (f64::from(bounds.max_y) - f64::from(bounds.min_y)) as f32,
            0.0,
        ];
        quad_collision_emit(list, &base, &[1.0, 0.0, 0.0], &e0_vert, &e1_horiz);
    }
}

/// One wall of the first three sides: `base`, `base + horiz`, `(base + horiz) + vert`, then `base`,
/// that corner and `base + vert`, each sum stored through `0x621160`. The plane is
/// `SetPlaneFromNormal`'s, or with `dot_plane` the normal as given and `d = −dot`.
fn emit_wall(
    list: &mut Vec<Facet>,
    normal_in: &[f32; 3],
    base: &[f32; 3],
    horiz: &[f32; 3],
    vert: &[f32; 3],
    dot_plane: bool,
) {
    let plane = if dot_plane {
        let d = (-dot3(*normal_in, *base)) as f32;
        [normal_in[0], normal_in[1], normal_in[2], d]
    } else {
        plane_from_np(normal_in, base)
    };
    let n = [plane[0], plane[1], plane[2]];
    let d = plane[3];

    let b_h = add3(base, horiz);
    let far = add3(&b_h, vert);
    let b_v = add3(base, vert);

    list.push(make_facet(n, d, *base, b_h, far));
    list.push(make_facet(n, d, *base, far, b_v));
}

/// `a + b`, each component stored as `f32`.
#[inline]
fn add3(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) + f64::from(b[0])) as f32,
        (f64::from(a[1]) + f64::from(b[1])) as f32,
        (f64::from(a[2]) + f64::from(b[2])) as f32,
    ]
}

/// One entry of the liquid batch at `0xc62550`, 0x20 bytes: the matrix pointer at `+0x0`, the
/// vertices (12 bytes apart) at `+0x4`, the index triples at `+0xc` and the triangle count, a
/// `u16`, at `+0x16`.
#[derive(Clone, Debug)]
pub struct LiquidBatch {
    /// The 4×4 matrix `0x7bca80` applies.
    pub matrix: [f32; 16],
    /// The vertices, three floats each.
    pub verts: Vec<f32>,
    /// The index triples.
    pub indices: Vec<u16>,
    /// The triangle count.
    pub tri_count: u16,
}

/// `FlushLiquidFacets` (`0x671cc0`), the path without SSE (`0xc9e388` clear): a facet for each
/// triangle of each entry, its vertices through `TransformPoint` (`0x7bca80`), the normal `(v1 − v0)
/// × (v2 − v0)` (`0x672130`) normalized (`0x6720f0`), and `d = −n·v0` (`0x602630`). With SSE the
/// normal is scaled by `rsqrtss`, which differs by CPU.
pub fn liquid_hit_flush(list: &mut Vec<Facet>, batches: &[LiquidBatch]) {
    for batch in batches {
        for t in 0..batch.tri_count as usize {
            let i0 = batch.indices[3 * t] as usize;
            let i1 = batch.indices[3 * t + 1] as usize;
            let i2 = batch.indices[3 * t + 2] as usize;
            let vin = |i: usize| {
                [
                    batch.verts[3 * i],
                    batch.verts[3 * i + 1],
                    batch.verts[3 * i + 2],
                ]
            };

            let v0 = transform_point(&vin(i0), &batch.matrix);
            let v1 = transform_point(&vin(i1), &batch.matrix);
            let v2 = transform_point(&vin(i2), &batch.matrix);

            // `0x4841f0` and `0x4549a0` take the edges.
            let e1 = sub3(&v1, &v0);
            let e2 = sub3(&v2, &v0);
            let mut n = vec3_cross_world(&e1, &e2);
            vec3_normalize_world(&mut n);
            let d = (-dot3(n, v0)) as f32;

            list.push(make_facet(n, d, v0, v1, v2));
        }
    }
}

/// `a − b`, each component stored as `f32`.
#[inline]
fn sub3(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) - f64::from(b[0])) as f32,
        (f64::from(a[1]) - f64::from(b[1])) as f32,
        (f64::from(a[2]) - f64::from(b[2])) as f32,
    ]
}
