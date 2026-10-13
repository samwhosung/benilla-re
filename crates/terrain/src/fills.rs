//! Vertex and table fills: the liquid's render vertices, the drift constants, the low-detail
//! tile's vertices, the chunk's index strips, the cell rectangle intersection and the chunk mesh's
//! coordinate tables.
//!
//! Some vertex slots are dwords copied as they are (positions, colours, heights), so the vertices
//! are `u32` arrays.

/// `[0x8029b0]`: 0.25, the liquid's texture step.
const MCLQ_UV: f64 = 0.25;
/// `[0x810388]`: 3/256 (`0x3c400000`), the magma's texture scale.
const TEXC: f32 = f32::from_bits(0x3c40_0000);
/// `[0x807b10]`: 8.0.
const MESH_UV8: f64 = 8.0;
/// `[0x802998]`: 1/16, where the inner vertices start.
const ONE_16: f64 = 0.0625;
/// `[0x802978]`: 1/8, the vertex step.
const ONE_8: f64 = 0.125;
/// `[0x80fee4]`: the chunk edge, 33.333332 (`0x42055555`).
const WDL_OUTER: f32 = f32::from_bits(0x4205_5555);
/// `[0x810e20]`: 16.666666 (`0x41855555`), half a chunk.
const WDL_INNER: f32 = f32::from_bits(0x4185_5555);
/// `[0x810334]`: 51/256 (`0x3e4c0000`).
const FX_UNIT: f32 = f32::from_bits(0x3e4c_0000);

/// A liquid render vertex of ten dwords: position, normal, two texture coordinates.
pub type RenderVert = [u32; 10];

/// The fill of `0x68d790` and `0x68d690`: for each of 81 vertices the position copied from the
/// liquid's `+0x34`, the normal (0, 0, 1), 0.5, `lut[d]` with `d` the vertex's first byte (`+0xc`,
/// 8 apart), then `f32(col·0.25)` and `f32(row·0.25)` (`fild` of the counters).
fn mclq_fill(pos: &[[u32; 3]; 81], depth: &[u8; 81], lut: &[u32; 256]) -> [RenderVert; 81] {
    let mut out = [[0u32; 10]; 81];
    let mut k = 0usize;
    for row in 0..9i32 {
        let v = ((row as i64 as f64) * MCLQ_UV) as f32;
        for col in 0..9i32 {
            let u = ((col as i64 as f64) * MCLQ_UV) as f32;
            let p = &pos[k];
            let vert = &mut out[k];
            vert[0] = p[0];
            vert[1] = p[1];
            vert[2] = p[2];
            vert[3] = 0; // normal x = 0.0
            vert[4] = 0; // normal y = 0.0
            vert[5] = 0x3f80_0000; // normal z = 1.0
            vert[6] = 0x3f00_0000; // 0.5
            vert[7] = lut[depth[k] as usize];
            vert[8] = u.to_bits();
            vert[9] = v.to_bits();
            k += 1;
        }
    }
    out
}

/// `CChunkLiquid::FillRiverVertices` (`0x68d790`, `systems/terrain/liquid.md#liquid-render-vertices`):
/// the fill with the table at `0xc81768`.
pub fn mclq_verts_a(
    pos: &[[u32; 3]; 81],
    depth: &[u8; 81],
    lut_c81768: &[u32; 256],
) -> [RenderVert; 81] {
    mclq_fill(pos, depth, lut_c81768)
}

/// `CChunkLiquid::FillOceanVertices` (`0x68d690`, `systems/terrain/liquid.md#liquid-render-vertices`):
/// the fill with the table at `0xc7fcd8`.
pub fn mclq_verts_b(
    pos: &[[u32; 3]; 81],
    depth: &[u8; 81],
    lut_c7fcd8: &[u32; 256],
) -> [RenderVert; 81] {
    mclq_fill(pos, depth, lut_c7fcd8)
}

/// `CChunkLiquid::FillMagmaVertices` (`0x68d890`, `systems/terrain/liquid.md#liquid-render-vertices`):
/// the position, the normal (0, 0, 1), `f32(w0·3/256)` and `f32(w1·3/256)` of the vertex's first two
/// `u16` at `+0xc` (8 apart, rows 0x48 apart), then 0 and 0.
pub fn liquid_render_verts(pos: &[[u32; 3]; 81], texcoords: &[[u16; 2]; 81]) -> [RenderVert; 81] {
    let mut out = [[0u32; 10]; 81];
    let mut k = 0usize;
    for _row in 0..9 {
        for _col in 0..9 {
            let p = &pos[k];
            let tc = &texcoords[k];
            let u = ((tc[0] as i32 as f64) * f64::from(TEXC)) as f32;
            let v = ((tc[1] as i32 as f64) * f64::from(TEXC)) as f32;
            let vert = &mut out[k];
            vert[0] = p[0];
            vert[1] = p[1];
            vert[2] = p[2];
            vert[3] = 0; // normal x = 0.0
            vert[4] = 0; // normal y = 0.0
            vert[5] = 0x3f80_0000; // normal z = 1.0
            vert[6] = u.to_bits();
            vert[7] = v.to_bits();
            vert[8] = 0;
            vert[9] = 0;
            k += 1;
        }
    }
    out
}

/// `InitTerrainDriftConstants` (`0x68ebf0`): 26 rectangles at `0xc81b98`, each component 0, 1, 2,
/// 3, 4 or 5 times 51/256: 0 as `u·0.0`, 1 copied, 2 as `u + u`, 3 to 5 as `u·3.0` (`0x80297c`),
/// `u·4.0` (`0x80306c`), `u·5.0` (`0x80a1e8`), each stored. In address order.
pub fn fx_uv_table() -> [f32; 104] {
    let z = (f64::from(FX_UNIT) * 0.0) as f32;
    let u1 = FX_UNIT;
    let u2 = (f64::from(FX_UNIT) + f64::from(FX_UNIT)) as f32;
    let u3 = (f64::from(FX_UNIT) * 3.0) as f32;
    let u4 = (f64::from(FX_UNIT) * 4.0) as f32;
    let u5 = (f64::from(FX_UNIT) * 5.0) as f32;

    // Each row's comment is its address.
    [
        z, z, z, u1, // c81b98
        u1, z, u1, u1, // c81ba8
        u1, z, u1, u1, // c81bb8
        u2, z, u2, u1, // c81bc8
        u2, z, u2, u1, // c81bd8
        u3, z, u3, u1, // c81be8
        u3, z, u3, u1, // c81bf8
        u4, z, u4, u1, // c81c08
        z, u1, z, u2, // c81c18
        u1, u1, u1, u2, // c81c28
        u1, u1, u1, u2, // c81c38
        u2, u1, u2, u2, // c81c48
        u2, u1, u2, u2, // c81c58
        u3, u1, u3, u2, // c81c68
        u3, u1, u3, u2, // c81c78
        u4, u1, u4, u2, // c81c88
        u4, z, u4, u1, // c81c98
        u5, z, u5, u1, // c81ca8
        z, u2, z, u3, // c81cb8
        u1, u2, u1, u3, // c81cc8
        u1, u2, u1, u3, // c81cd8
        u2, u2, u2, u3, // c81ce8
        u2, u2, u2, u3, // c81cf8
        u3, u2, u3, u3, // c81d08
        u3, u2, u3, u3, // c81d18
        u4, u2, u4, u3, // c81d28
    ]
}

/// A low-detail vertex: x, y, the height copied as a dword, and 0xffffffff.
pub type WdlVert = [u32; 4];

/// `CMapAreaLow::FillVertices` (`0x6bd8c0`, `systems/terrain/lod.md#low-detail-mesh`): into the
/// locked buffer, the 17×17 outer vertices from `(+0x28, +0x2c)` down 33.333332 a row (x) and a
/// column (y), heights from `+0x3c`, then the 16×16 inner ones from 16.666666 less, heights from
/// `+0x4c0`. Each coordinate is stored as `f32` (`fst`) while the running value stays at 53 bits.
pub fn wdl_mesh_fill(
    x0: f32,
    y0: f32,
    outer_heights: &[f32; 289], // +0x3c, 17×17
    inner_heights: &[f32; 256], // +0x4c0, 16×16
) -> Vec<WdlVert> {
    let mut out: Vec<WdlVert> = Vec::with_capacity(289 + 256);
    let w = 0xffff_ffffu32;

    let mut x = f64::from(x0);
    for i in 0..17usize {
        let mut y = f64::from(y0);
        for j in 0..17usize {
            let xf = x as f32;
            let yf = y as f32;
            y -= f64::from(WDL_OUTER);
            out.push([
                xf.to_bits(),
                yf.to_bits(),
                outer_heights[i * 17 + j].to_bits(),
                w,
            ]);
        }
        x -= f64::from(WDL_OUTER);
    }

    let mut x = f64::from(x0) - f64::from(WDL_INNER);
    for i in 0..16usize {
        let mut y = f64::from(y0) - f64::from(WDL_INNER);
        for j in 0..16usize {
            let xf = x as f32;
            let yf = y as f32;
            y -= f64::from(WDL_OUTER);
            out.push([
                xf.to_bits(),
                yf.to_bits(),
                inner_heights[i * 16 + j].to_bits(),
                w,
            ]);
        }
        x -= f64::from(WDL_OUTER);
    }
    out
}

/// `0x8103f8`: the hole bit of each 2×2 block of cells.
const HOLES_TABLE: [u16; 16] = [
    1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];

/// The spans per level: `u16` at `0x811230` and `0x81122c`, 8 bytes apart.
const SPAN_1230: [u16; 4] = [8, 4, 2, 1];
const SPAN_122C: [u16; 4] = [136, 68, 34, 17];

/// The children's offsets, `[4·level + child]`: x at `0x8111cc`, y at `0x8111fc`; level 3 is
/// always a leaf, so its y row, which overlaps the span table, is never read.
const CHILD_DX: [i32; 16] = [-2, 2, 2, -2, -1, 1, 1, -1, -1, 0, 0, -1, -2, -2, 2, 2];
const CHILD_DY: [i32; 16] = [-2, -2, 2, 2, -1, -1, 1, 1, -1, -1, 0, 0, 0, 0, 0, 0];

/// `BuildChunkIndices` (`0x6c51b0`, `systems/terrain/lod.md#chunk-index-building`, ecx = level, edx
/// = cap, then caps, holes, x, y): with holes at level 1, the four children not in a hole (table bit
/// of `((x' − 1) >> 1) + 4·((y' − 1) >> 1)`) at level 2; below `cap`, all four children a level
/// down; otherwise a leaf, whose `i16` indices go through the cursor at `0xcac3ac`, four edge fans
/// each split when the level is below the matching byte of `caps`. The children's `caps` are woven
/// from `cap` and `caps` at entry.
pub fn quad_hole_strip_indices(
    level: u32,
    cap: u32,
    caps: u32,
    holes: u32,
    gx: i32,
    gy: i32,
    out: &mut Vec<i16>,
) {
    // The children's caps, from `0x6c51bd`.
    let uvar4 = (cap << 8) | cap;
    let local_18 = [
        (uvar4 << 8) | (caps & 0xff00_00ff),
        (uvar4 << 0x10) | (caps & 0xffff),
        (cap << 0x18) | (caps & 0xff_ff00) | cap,
        (caps & 0xffff_0000) | (cap << 8) | cap,
    ];

    // Holes at level 1: the level-1 offsets, entries 4 to 7.
    if holes != 0 && level == 1 {
        for (child, &caps_child) in local_18.iter().enumerate() {
            let t = 4 + child;
            let new_gy = CHILD_DY[t] + gy;
            let new_gx = CHILD_DX[t] + gx;
            // `lea -1(eax); shr` and `lea (ebx,edx,4)`; the caller's coordinates are at least 1.
            let hx = (new_gx as u32).wrapping_sub(1) >> 1;
            let hy = (new_gy as u32).wrapping_sub(1) >> 1;
            let idx = hx.wrapping_add(hy.wrapping_mul(4)) as usize;
            if u32::from(HOLES_TABLE[idx]) & holes == 0 {
                quad_hole_strip_indices(2, cap, caps_child, holes, new_gx, new_gy, out);
            }
        }
        return;
    }

    // Below `cap`, signed.
    if (level as i32) < (cap as i32) {
        for (child, &caps_child) in local_18.iter().enumerate() {
            let t = (level as usize) * 4 + child;
            let new_gy = CHILD_DY[t] + gy;
            let new_gx = CHILD_DX[t] + gx;
            quad_hole_strip_indices(level + 1, cap, caps_child, holes, new_gx, new_gy, out);
        }
        return;
    }

    // The leaf: `y·0x11` in 16 bits.
    let uvar1 = i32::from(SPAN_1230[level as usize]);
    let uvar2 = i32::from(SPAN_122C[level as usize]);
    let mut sv7 = i32::from((gy as i16).wrapping_mul(0x11));
    let sv3: i32;
    if level == 3 {
        sv7 = sv7.wrapping_add(gx);
        sv3 = sv7.wrapping_add(9);
    } else {
        sv3 = sv7.wrapping_add(gx);
        sv7 = sv3
            .wrapping_sub(i32::from(uvar2 as u16 >> 1))
            .wrapping_sub(i32::from(uvar1 as u16 >> 1));
    }
    // The first value is written at the cursor, every later one after advancing it by 2.
    let push = |out: &mut Vec<i16>, v: i32| out.push(v as i16);

    push(out, sv3);
    if level < (caps >> 0x18) {
        push(out, sv7);
        let sv5 = i32::from(uvar2 as u16 >> 1) + sv7;
        push(out, sv5);
        push(out, sv3);
        push(out, sv5);
        push(out, sv7 + uvar2);
    } else {
        push(out, sv7);
        push(out, sv7 + uvar2);
    }
    push(out, sv3);
    if level < ((caps >> 0x10) & 0xff) {
        push(out, sv7 + uvar2);
        let sv5 = i32::from(uvar1 as u16 >> 1) + sv7 + uvar2;
        push(out, sv5);
        push(out, sv3);
        push(out, sv5);
    } else {
        push(out, sv7 + uvar2);
    }
    let mut sv5 = sv7 + uvar2 + uvar1;
    push(out, sv5);
    if level < ((caps >> 8) & 0xff) {
        push(out, sv3);
        push(out, sv5);
        sv5 = i32::from(uvar2 as u16 >> 1) + sv7 + uvar1;
        push(out, sv5);
        push(out, sv3);
    } else {
        push(out, sv3);
    }
    push(out, sv5);
    sv5 = sv7 + uvar1;
    push(out, sv5);
    push(out, sv3);
    if level < (caps & 0xff) {
        push(out, sv5);
        let sv5b = i32::from(uvar1 as u16 >> 1) + sv7;
        push(out, sv5b);
        push(out, sv3);
        push(out, sv5b);
    } else {
        push(out, sv5);
    }
    push(out, sv7);
}

/// `IntersectCellRects` (`0x6adbf0`, `systems/terrain/collide.md#other-world-queries`, `out`, `b`,
/// `c`): `[max, max, min, min]` of two `[min x, min y, max x, max y]` rectangles; on a tie the first
/// two take `c` (`jg`, `jle`) and the last two `b` (`jl`). An empty result is returned as it is.
pub fn aabb_intersect_i(b: [i32; 4], c: [i32; 4]) -> [i32; 4] {
    let o0 = if b[0] > c[0] { b[0] } else { c[0] };
    let o1 = if b[1] <= c[1] { c[1] } else { b[1] };
    let o2 = if c[2] < b[2] { c[2] } else { b[2] };
    let o3 = if c[3] < b[3] { c[3] } else { b[3] };
    [o0, o1, o2, o3]
}

/// The two tables `0x6aef50` builds, in the vertices' order: 9 outer then 8 inner a row.
pub struct ChunkMeshTables {
    /// `0xcac860`: the coordinates times 8.
    pub uv: [[f32; 2]; 145],
    /// `0xcac3c8`: the coordinates in `[0, 1]`, outer at `k/8`, inner at `1/16 + k/8`.
    pub pos: [[f32; 2]; 145],
}

/// `BuildChunkMeshTables` (`0x6aef50`, `systems/terrain/lod.md#chunk-mesh-tables`), its two
/// coordinate tables: the coordinates step by 1/8 from 0, or from 1/16 for the inner rows and
/// columns, at 53 bits; `0x6aef89` multiplies by 8 before each store into `0xcac860` and `0x6af020`
/// stores the value into `0xcac3c8`. The copies per level after them are not modelled.
pub fn chunk_mesh_tables() -> ChunkMeshTables {
    let mut uv = [[0f32; 2]; 145];
    let mut pos = [[0f32; 2]; 145];
    fill_grid(&mut uv, MESH_UV8);
    fill_grid(&mut pos, 1.0);
    ChunkMeshTables { uv, pos }
}

/// The rows of a table, each component `f32(c·scale)`.
fn fill_grid(table: &mut [[f32; 2]; 145], scale: f64) {
    let mut outer = 0.0f64;
    let mut inner = ONE_16;
    let mut k = 0usize;
    for i in 0..9i32 {
        let v = (outer * scale) as f32;
        let mut col = 0.0f64;
        for _ in 0..9 {
            let u = (col * scale) as f32;
            table[k] = [u, v];
            k += 1;
            col += ONE_8;
        }
        outer += ONE_8;
        if i < 8 {
            let vi = (inner * scale) as f32;
            let mut col = ONE_16;
            for _ in 0..8 {
                let u = (col * scale) as f32;
                table[k] = [u, vi];
                k += 1;
                col += ONE_8;
            }
            inner += ONE_8;
        }
    }
}
