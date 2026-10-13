//! The chunk gathers, each run whole on a chunk the test lays out: `TestChunkTriangles`
//! (`0x6ad7e0`), `GatherLiquidCellFacets` (`0x68cec0`), and on their terrain part without SSE
//! `GatherChunkFacets` (`0x6aadc0`), through `PlaneFromTriangle` (`0x637480`), and
//! `GatherChunkVolumeFacets` (`0x6ac1e0`), through `0x637480` and `GetPlaneOutcode` (`0x686c20`).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::gather::{
    chunk_collect_aabb, chunk_gather_aabb, chunk_gather_planes, mclq_collect, BatchHeader,
    BatchProtocol, ChunkModel, FacetArray, PlaneRecord,
};
use common::assert_f32;

const AREA_TABLE: u64 = 0x00c9_6318; // the area table

const DAT_C9E388: u64 = 0x00c9_e388; // the SSE switch

// The batch's globals.
const DAT_C5A474: u64 = 0x00c5_a474; // header count

const DAT_C62550: u64 = 0x00c6_2550; // headers

const DAT_C63190: u64 = 0x00c6_3190; // matrix count

const DAT_C63298: u64 = 0x00c6_3298; // indices

const DAT_C7B378: u64 = 0x00c7_b378; // index cursor

const DAT_C63258: u64 = 0x00c6_3258; // overflow flag

const DAT_C7B2A4: u64 = 0x00c7_b2a4; // debug drawing, 0 so it never runs

/// Writes floats to consecutive dwords.
fn write_f32s(emu: &mut Emu, base: u64, vals: &[f32]) {
    for (k, v) in vals.iter().enumerate() {
        emu.write_u32(base + 4 * k as u64, v.to_bits());
    }
}

/// Writes integers to consecutive dwords.
fn write_i32s(emu: &mut Emu, base: u64, vals: &[i32]) {
    for (k, v) in vals.iter().enumerate() {
        emu.write_u32(base + 4 * k as u64, *v as u32);
    }
}

/// The LCG; choices take the high bits.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

/// Lays out a chunk at `chunk_base` at position 0, so world and chunk coordinates agree, with the
/// given holes and random heights for its 145 vertices (`+0x83c`); returns the vertices.
fn write_chunk(
    emu: &mut Emu,
    chunk_base: u64,
    holes: u16,
    flags: u32,
    rng: &mut Lcg,
) -> Vec<[f32; 3]> {
    // +0xf08 points at a block whose +0x3c is the hole word
    let hdr = chunk_base + 0x4000;
    emu.write_u32(chunk_base + 0xf08, hdr as u32);
    emu.write_u32(hdr + 0x3c, u32::from(holes)); // a u16 is read; the word is written low
                                                 // the flags at +0xc
    emu.write_u32(chunk_base + 0xc, flags);
    // the position at +0x6c, 0
    for k in 0..3 {
        emu.write_u32(chunk_base + 0x6c + 4 * k, 0);
    }
    // the vertices at +0x83c, 12 bytes apart: 145 end at 0xac0, below +0xf08
    let mut verts = vec![[0f32; 3]; 145];
    for (i, v) in verts.iter_mut().enumerate() {
        // x and y from the index, z random
        let col = (i % 17) as f32;
        let row = (i / 17) as f32;
        let z = (rng.next() >> 8) as f32 / 4096.0 - 8.0;
        *v = [col * 2.0, row * 2.0, z];
        let off = chunk_base + 0x83c + 0xc * i as u64;
        for (k, &comp) in v.iter().enumerate() {
            emu.write_u32(off + 4 * k as u64, comp.to_bits());
        }
    }
    // no doodads: +0xe4 = 0, and +0xdc = 0
    emu.write_u32(chunk_base + 0xe4, 0);
    emu.write_u32(chunk_base + 0xdc, 0);
    // no liquids or models from +0x118
    for k in 0..4u64 {
        emu.write_u32(chunk_base + 0x118 + 4 * k, 0);
    }
    verts
}

/// Sets the area table so tile (0, 0) has the chunk.
fn wire_area_table(emu: &mut Emu, tile_base: u64, chunk_base: u64) {
    emu.write_u32(AREA_TABLE, tile_base as u32);
    emu.write_u32(tile_base + 0x278, chunk_base as u32);
}

/// Clears the batch's globals, as `0x6ab4e0` does before a walk.
fn reset_batch(emu: &mut Emu) {
    emu.write_u32(DAT_C5A474, 0);
    emu.write_u32(DAT_C63190, 0);
    emu.write_u32(DAT_C7B378, 0);
    emu.write_u32(DAT_C63258, 0);
    emu.write_u32(DAT_C7B2A4, 0);
}

/// The batch as the exe left it: each header's index pointer (`+0xc`) as a slot, `(ptr −
/// 0xc63298)/2`, and its written indices only, as the reserved span can run past them.
fn read_batch(emu: &Emu) -> (Vec<BatchHeader>, Vec<Vec<u16>>, u32, bool) {
    let nhdr = emu.read_u32(DAT_C5A474);
    let mut headers = Vec::new();
    let mut written = Vec::new();
    for h in 0..nhdr as u64 {
        let rec = DAT_C62550 + h * 0x20;
        let lo16 = |va: u64| (emu.read_u32(va) & 0xffff) as u16;
        let idx_ptr = emu.read_u32(rec + 0xc) as u64;
        let index_base = if idx_ptr >= DAT_C63298 {
            ((idx_ptr - DAT_C63298) / 2) as u32
        } else {
            0
        };
        let count = lo16(rec + 0x14);
        let hdr = BatchHeader {
            verts_base: emu.read_u32(rec + 0x4),
            normals_base: emu.read_u32(rec + 0x8),
            owner: emu.read_u32(rec + 0x1c),
            index_base,
            index_count: count,
            tri_count: lo16(rec + 0x16),
            index_min: lo16(rec + 0x18),
            index_max: lo16(rec + 0x1a),
        };
        let mut idxs = Vec::new();
        for k in 0..count as u64 {
            idxs.push((emu.read_u32(DAT_C63298 + 2 * (u64::from(index_base) + k)) & 0xffff) as u16);
        }
        headers.push(hdr);
        written.push(idxs);
    }
    (
        headers,
        written,
        emu.read_u32(DAT_C63190),
        emu.read_u32(DAT_C63258) & 1 != 0,
    )
}

fn assert_batch_eq(
    bin: &(Vec<BatchHeader>, Vec<Vec<u16>>, u32, bool),
    ours: &BatchProtocol,
    ctx: &str,
) {
    let (bin_hdrs, bin_idx, bin_mtx, bin_ovf) = bin;
    assert_eq!(bin_hdrs.len(), ours.headers.len(), "{ctx}: header count");
    for (i, (b, o)) in bin_hdrs.iter().zip(ours.headers.iter()).enumerate() {
        assert_eq!(b, o, "{ctx}: header[{i}]");
        assert_eq!(
            &bin_idx[i],
            &ours.written_indices(o),
            "{ctx}: header[{i}] indices"
        );
    }
    assert_eq!(*bin_mtx, ours.matrix_count, "{ctx}: matrix count");
    assert_eq!(*bin_ovf, ours.overflow, "{ctx}: overflow");
}

#[test]
fn chunk_collect_aabb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let chunk_base = emu.alloc(0x6000);
    let tile_base = emu.alloc(0x2000);
    let aabb_p = emu.alloc(0x20);
    let rect_p = emu.alloc(0x20);

    // the paths taken
    let (mut emit_cases, mut skip_cases, mut hole_cases) = (0u32, 0u32, 0u32);

    let mut rng = Lcg(0xc0ff_ee11);
    // (holes, rectangle, box): cases across the paths
    let cases: Vec<(u16, [i32; 4], [f32; 6])> = vec![
        // every cell, no holes, a box over everything
        (0x0000, [0, 0, 7, 7], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
        // a hole on cell (0, 0)
        (0x0001, [0, 0, 7, 7], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
        // a thin z slab, so most triangles share a bit
        (0x0000, [0, 0, 7, 7], [-1e9, -1e9, 100.0, 1e9, 1e9, 200.0]),
        // a rectangle inside the chunk; this function does not clamp, the caller does
        (0x0000, [1, 2, 5, 6], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
        // one cell
        (0x0000, [3, 3, 3, 3], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
        // holes over several rows
        (0x0a0a, [0, 0, 7, 7], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
    ];

    for (ci, (holes, rect, aabb)) in cases.iter().enumerate() {
        let verts = write_chunk(&mut emu, chunk_base, *holes, 0x0, &mut rng);
        wire_area_table(&mut emu, tile_base, chunk_base);
        reset_batch(&mut emu);
        write_f32s(&mut emu, aabb_p, aabb);
        write_i32s(&mut emu, rect_p, rect);
        // ecx = the chunk, edx = the rectangle (already within 0 to 7), the box on the stack
        emu.call(
            0x006a_d7e0,
            chunk_base as u32,
            rect_p as u32,
            &[aabb_p as u32],
        );
        let bin = read_batch(&emu);

        // the reimplementation
        let chunk = ChunkModel {
            flags: 0,
            origin: [0.0, 0.0, 0.0],
            normals_off: chunk_base as u32 + 0x170,
            verts: &verts,
            holes: *holes,
        };
        let mut ours = BatchProtocol::default();
        let verts_base = chunk_base as u32 + 0x83c;
        // the header's owner (+0x1c) is the chunk
        chunk_collect_aabb(
            &chunk,
            aabb,
            *rect,
            verts_base,
            chunk_base as u32,
            &mut ours,
        );

        assert_batch_eq(&bin, &ours, &format!("collect_aabb case#{ci}"));

        // the paths
        let (bin_hdrs, bin_idx, _, _) = &bin;
        if *holes != 0 {
            hole_cases += 1;
        }
        if !bin_hdrs.is_empty() && bin_hdrs[0].tri_count < 4 * 64 {
            skip_cases += 1;
        }
        if bin_idx.iter().any(|v| !v.is_empty()) {
            emit_cases += 1;
        }
    }
    assert!(emit_cases > 0, "no emit case fired");
    assert!(skip_cases > 0, "no shared-outcode skip fired");
    assert!(hole_cases > 0, "no hole case fired");

    // Two calls without a reset: the second header's index_base is the first's whole span, so
    // the span's formula is compared, which a single call (base 0) cannot show. The rectangles
    // differ so the spans do.
    {
        let verts = write_chunk(&mut emu, chunk_base, 0x0000, 0x0, &mut rng);
        wire_area_table(&mut emu, tile_base, chunk_base);
        reset_batch(&mut emu);
        let big = [-1e9f32, -1e9, -1e9, 1e9, 1e9, 1e9];
        write_f32s(&mut emu, aabb_p, &big);
        let chunk = ChunkModel {
            flags: 0,
            origin: [0.0, 0.0, 0.0],
            normals_off: chunk_base as u32 + 0x170,
            verts: &verts,
            holes: 0x0000,
        };
        let verts_base = chunk_base as u32 + 0x83c;
        let mut ours = BatchProtocol::default();
        for rect in [[0i32, 0, 2, 3], [1i32, 1, 6, 5]] {
            write_i32s(&mut emu, rect_p, &rect);
            emu.call(
                0x006a_d7e0,
                chunk_base as u32,
                rect_p as u32,
                &[aabb_p as u32],
            );
            chunk_collect_aabb(&chunk, &big, rect, verts_base, chunk_base as u32, &mut ours);
        }
        let bin = read_batch(&emu);
        assert_eq!(bin.0.len(), 2, "two-call: expected 2 headers");
        assert!(
            bin.0[1].index_base > 0,
            "two-call: second index_base must reflect the first span reservation"
        );
        assert_batch_eq(&bin, &ours, "collect_aabb two-call span");
    }
}

#[test]
fn chunk_collect_aabb_overflow_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let chunk_base = emu.alloc(0x6000);
    let tile_base = emu.alloc(0x2000);
    let aabb_p = emu.alloc(0x20);
    let rect_p = emu.alloc(0x20);

    let mut rng = Lcg(0xfeed_face);
    let verts = write_chunk(&mut emu, chunk_base, 0, 0x0, &mut rng);
    wire_area_table(&mut emu, tile_base, chunk_base);
    reset_batch(&mut emu);
    // the cursor just below 0xc000, so the span overflows
    emu.write_u32(DAT_C7B378, 0xc000 - 4);
    let aabb = [-1e9f32, -1e9, -1e9, 1e9, 1e9, 1e9];
    let rect = [0i32, 0, 7, 7];
    write_f32s(&mut emu, aabb_p, &aabb);
    write_i32s(&mut emu, rect_p, &rect);
    emu.call(
        0x006a_d7e0,
        chunk_base as u32,
        rect_p as u32,
        &[aabb_p as u32],
    );
    let (_, _, _, bin_ovf) = read_batch(&emu);
    assert!(bin_ovf, "binary did not flag index overflow");

    let chunk = ChunkModel {
        flags: 0,
        origin: [0.0, 0.0, 0.0],
        normals_off: chunk_base as u32 + 0x170,
        verts: &verts,
        holes: 0,
    };
    let mut ours = BatchProtocol::default();
    // the same cursor for the reimplementation
    ours.index_cursor = 0xc000 - 4;
    chunk_collect_aabb(&chunk, &aabb, rect, chunk_base as u32 + 0x83c, 0, &mut ours);
    assert_eq!(ours.overflow, bin_ovf, "overflow flag mismatch");
}

#[test]
fn mclq_collect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // ecx = the liquid's container: +0x10 the type bytes, +0x1c the chunk (its position at +0x6c),
    // +0x34 the vertices (12 bytes apart, z at +8)
    let this_p = emu.alloc(0x2000);
    let flags_p = emu.alloc(0x100);
    let chunk_p = emu.alloc(0x1000);
    let query_p = emu.alloc(0x20);
    let rect_p = emu.alloc(0x20);

    // the chunk at position 0
    emu.write_u32(this_p + 0x1c, chunk_p as u32);
    for k in 0..3u64 {
        emu.write_u32(chunk_p + 0x6c + 4 * k, 0);
    }
    emu.write_u32(this_p + 0x10, flags_p as u32);

    let mut rng = Lcg(0xabcd_0001);
    let cases: Vec<(Vec<u8>, [i32; 4], [f32; 6])> = vec![
        // every cell open, a band over every z
        (vec![0u8; 64], [0, 0, 7, 7], [0.0, 0.0, -1e9, 0.0, 0.0, 1e9]),
        // some cells 0xf, skipped
        (
            {
                let mut f = vec![0u8; 64];
                f[0] = 0xf;
                f[9] = 0xf;
                f
            },
            [0, 0, 7, 7],
            [0.0, 0.0, -1e9, 0.0, 0.0, 1e9],
        ),
        // a thin z band, so most share a bit
        (vec![0u8; 64], [0, 0, 7, 7], [0.0, 0.0, 2.0, 0.0, 0.0, 3.0]),
        // a part of the grid
        (vec![0u8; 64], [2, 1, 5, 6], [0.0, 0.0, -1e9, 0.0, 0.0, 1e9]),
    ];

    let mut emit_cases = 0u32;
    let mut skip_cases = 0u32;
    for (ci, (flags, rect, query)) in cases.iter().enumerate() {
        // the 9×9 vertices at +0x34, z random
        let mut verts_z = [0f32; 81];
        let layer = this_p + 0x34;
        for (i, z) in verts_z.iter_mut().enumerate() {
            let zz = (rng.next() >> 8) as f32 / 4096.0 - 2.0;
            *z = zz;
            let off = layer + 0xc * i as u64;
            emu.write_u32(off, 0); // x
            emu.write_u32(off + 4, 0); // y
            emu.write_u32(off + 8, zz.to_bits()); // z
        }
        for (i, &b) in flags.iter().enumerate() {
            emu.write(flags_p + i as u64, &[b]);
        }
        write_f32s(&mut emu, query_p, query);
        write_i32s(&mut emu, rect_p, rect);
        reset_batch(&mut emu);
        // ecx = this, then a scratch argument, &query and &rect
        emu.call(
            0x0068_cec0,
            this_p as u32,
            0,
            &[0, query_p as u32, rect_p as u32],
        );
        let bin = read_batch(&emu);

        let mut flags_arr = [0u8; 64];
        flags_arr.copy_from_slice(flags);
        let mut ours = BatchProtocol::default();
        let layer_verts_base = layer as u32;
        let owner = emu.read_u32(this_p + 0x1c); // the header's owner is the chunk at +0x1c
        mclq_collect(
            layer_verts_base,
            owner,
            query,
            &flags_arr,
            &verts_z,
            *rect,
            &mut ours,
        );
        assert_batch_eq(&bin, &ours, &format!("mclq case#{ci}"));

        let (bin_hdrs, bin_idx, _, _) = &bin;
        if bin_idx.iter().any(|v| !v.is_empty()) {
            emit_cases += 1;
        }
        if !bin_hdrs.is_empty() && bin_hdrs[0].tri_count < 2 * 64 {
            skip_cases += 1;
        }
    }
    assert!(emit_cases > 0, "no mclq emit fired");
    assert!(skip_cases > 0, "no mclq shared-outcode skip fired");
}

#[test]
fn chunk_gather_aabb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let chunk_base = emu.alloc(0x6000);
    let tile_base = emu.alloc(0x2000);
    let aabb_p = emu.alloc(0x40); // 6 floats; the function copies them with `rep movsd`
    let rect_p = emu.alloc(0x20);
    // The facet list: capacity at +0, count at +4, base at +8; +0x18 is the 8-byte array the
    // function zeroes after the walk, given real memory.
    let facet_arr = emu.alloc(0x40);
    let facet_store = emu.alloc(0x34 * 0x400); // large, so 0x6742d0 never reallocates
    let facet_aux = emu.alloc(0x8 * 0x800); // the +0x18 array

    emu.write_u32(DAT_C9E388, 0); // the x87 path

    let mut rng = Lcg(0x5151_2323);
    let cases: Vec<(u16, [i32; 4], [f32; 6])> = vec![
        (0x0000, [0, 0, 7, 7], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
        (0x0004, [0, 0, 7, 7], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
        (0x0000, [0, 0, 7, 7], [-1e9, -1e9, 100.0, 1e9, 1e9, 110.0]),
        (0x0000, [-2, -1, 10, 9], [-1e9, -1e9, -1e9, 1e9, 1e9, 1e9]),
    ];

    let mut emit_cases = 0u32;
    let mut skip_cases = 0u32;
    let mut hole_cases = 0u32;
    for (ci, (holes, rect, aabb)) in cases.iter().enumerate() {
        let verts = write_chunk(&mut emu, chunk_base, *holes, 0x0, &mut rng);
        wire_area_table(&mut emu, tile_base, chunk_base);
        // the list empty, its capacity large
        emu.write_u32(facet_arr, 0x400);
        emu.write_u32(facet_arr + 0x4, 0);
        emu.write_u32(facet_arr + 0x8, facet_store as u32);
        emu.write_u32(facet_arr + 0xc, 0);
        // +0x10 and +0x14 above any count, so the hand-off after the walk (0x674520, 0x6acc90,
        // 0x613750) does not run
        emu.write_u32(facet_arr + 0x10, 0xffff_ffff);
        emu.write_u32(facet_arr + 0x14, 0xffff_ffff);
        emu.write_u32(facet_arr + 0x18, facet_aux as u32);
        emu.write_u32(facet_arr + 0x1c, 0);
        for b in 0..(0x34 * 0x400u64) / 4 {
            emu.write_u32(facet_store + 4 * b, 0);
        }
        // the +0x18 array filled first, so its zeroing shows
        for b in 0..(0x8 * 0x800u64) / 4 {
            emu.write_u32(facet_aux + 4 * b, 0xaaaa_aaaa);
        }
        // the box and the rectangle
        write_f32s(&mut emu, aabb_p, aabb);
        write_i32s(&mut emu, rect_p, rect);
        // ecx = grid x 0, edx = grid y 0, then &rect, &box, &list and the mask 0xf00: the terrain
        // only, and no impassable flag
        emu.write_u32(DAT_C7B2A4, 0);
        emu.call(
            0x006a_adc0,
            0,
            0,
            &[rect_p as u32, aabb_p as u32, facet_arr as u32, 0xf00],
        );
        let n = emu.read_u32(facet_arr + 0x4);
        let mut bin = Vec::new();
        for r in 0..n as u64 {
            let rec = facet_store + r * 0x34;
            let f = |o: u64| emu.read_f32(rec + o);
            bin.push(PlaneRecord {
                plane: [f(0x0), f(0x4), f(0x8), f(0xc)],
                v0: [f(0x10), f(0x14), f(0x18)],
                v1: [f(0x1c), f(0x20), f(0x24)],
                v2: [f(0x28), f(0x2c), f(0x30)],
            });
        }
        // the +0x18 array and its count at +0x14
        let bin_rowcount = emu.read_u32(facet_arr + 0x14);
        let bin_rows: Vec<[f32; 2]> = (0..n as u64)
            .map(|r| {
                [
                    emu.read_f32(facet_aux + r * 8),
                    emu.read_f32(facet_aux + r * 8 + 4),
                ]
            })
            .collect();

        let chunk = ChunkModel {
            flags: 0,
            origin: [0.0, 0.0, 0.0],
            normals_off: chunk_base as u32 + 0x170,
            verts: &verts,
            holes: *holes,
        };
        let mut ours = FacetArray::default();
        chunk_gather_aabb(&chunk, aabb, *rect, &mut ours, 0);

        assert_eq!(
            bin.len(),
            ours.records.len(),
            "gather_aabb case#{ci}: record count"
        );
        // the array grows to the count and is zeroed
        assert_eq!(
            bin_rowcount, n,
            "gather_aabb case#{ci}: rows count (+0x14) == record count"
        );
        assert_eq!(
            bin_rows.len(),
            ours.rows.len(),
            "gather_aabb case#{ci}: rows len"
        );
        for (i, (b, o)) in bin_rows.iter().zip(ours.rows.iter()).enumerate() {
            assert_f32(b[0], o[0], &format!("gather_aabb case#{ci} row[{i}].x"));
            assert_f32(b[1], o[1], &format!("gather_aabb case#{ci} row[{i}].y"));
        }
        for (i, (b, o)) in bin.iter().zip(ours.records.iter()).enumerate() {
            for k in 0..4 {
                assert_f32(
                    b.plane[k],
                    o.plane[k],
                    &format!("case#{ci} rec[{i}].plane[{k}]"),
                );
            }
            for k in 0..3 {
                assert_f32(b.v0[k], o.v0[k], &format!("case#{ci} rec[{i}].v0[{k}]"));
                assert_f32(b.v1[k], o.v1[k], &format!("case#{ci} rec[{i}].v1[{k}]"));
                assert_f32(b.v2[k], o.v2[k], &format!("case#{ci} rec[{i}].v2[{k}]"));
            }
        }
        if !bin.is_empty() {
            emit_cases += 1;
        }
        if bin.len() < 4 * 64 {
            skip_cases += 1;
        }
        if *holes != 0 {
            hole_cases += 1;
        }
    }
    assert!(emit_cases > 0 && skip_cases > 0 && hole_cases > 0);
}

/// A chunk at a position like a real one: `0x6aadc0` takes the whole box relative to the chunk,
/// its minimum inline and its maximum through `0x4841c0`, so the box's maximum is compared with the
/// chunk's own heights. A chunk at position 0 cannot show the maximum's part; these cases do.
#[test]
fn chunk_gather_aabb_nonzero_origin_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let chunk_base = emu.alloc(0x6000);
    let tile_base = emu.alloc(0x2000);
    let aabb_p = emu.alloc(0x40);
    let rect_p = emu.alloc(0x20);
    let facet_arr = emu.alloc(0x40);
    let facet_store = emu.alloc(0x34 * 0x400);
    let facet_aux = emu.alloc(0x8 * 0x800);

    emu.write_u32(DAT_C9E388, 0); // the x87 path

    // The position's z is far below zero and the vertices' own z large (about the world z less the
    // position), so a box near the player's world height lies below them in world terms: only a
    // maximum taken relative to the chunk keeps them.
    let origin = [2114.0f32, 3714.0, -501.0];
    // Boxes in world coordinates around the chunk: case 0's z from −50 to 100 is 451 to 601 for the
    // chunk and covers every vertex (548 to 555); case 1's maximum 53 is 554 and clips the vertices
    // above it.
    let cases: Vec<(u16, [i32; 4], [f32; 6])> = vec![
        (
            0x0000,
            [0, 0, 7, 7],
            [2100.0, 3700.0, -50.0, 2160.0, 3740.0, 100.0],
        ),
        (
            0x0000,
            [0, 0, 7, 7],
            [2100.0, 3700.0, -50.0, 2160.0, 3740.0, 53.0],
        ),
    ];

    let mut rng = Lcg(0x77aa_1234);
    for (ci, (holes, rect, aabb)) in cases.iter().enumerate() {
        let mut verts = write_chunk(&mut emu, chunk_base, *holes, 0x0, &mut rng);
        // heights from 548 to 555, in the emulator and the model alike, so the maximum z decides
        for (i, v) in verts.iter_mut().enumerate() {
            let z = 548.0 + (i % 8) as f32; // 548 to 555
            v[2] = z;
            emu.write_u32(chunk_base + 0x83c + 0xc * i as u64 + 8, z.to_bits());
        }
        // the position, which write_chunk set to 0
        for k in 0..3u64 {
            emu.write_u32(chunk_base + 0x6c + 4 * k, origin[k as usize].to_bits());
        }
        wire_area_table(&mut emu, tile_base, chunk_base);
        emu.write_u32(facet_arr, 0x400);
        emu.write_u32(facet_arr + 0x4, 0);
        emu.write_u32(facet_arr + 0x8, facet_store as u32);
        emu.write_u32(facet_arr + 0xc, 0);
        emu.write_u32(facet_arr + 0x10, 0xffff_ffff);
        emu.write_u32(facet_arr + 0x14, 0xffff_ffff);
        emu.write_u32(facet_arr + 0x18, facet_aux as u32);
        emu.write_u32(facet_arr + 0x1c, 0);
        for b in 0..(0x34 * 0x400u64) / 4 {
            emu.write_u32(facet_store + 4 * b, 0);
        }
        write_f32s(&mut emu, aabb_p, aabb);
        write_i32s(&mut emu, rect_p, rect);
        emu.write_u32(DAT_C7B2A4, 0);
        emu.call(
            0x006a_adc0,
            0,
            0,
            &[rect_p as u32, aabb_p as u32, facet_arr as u32, 0xf00],
        );
        let n = emu.read_u32(facet_arr + 0x4);
        let mut bin = Vec::new();
        for r in 0..n as u64 {
            let rec = facet_store + r * 0x34;
            let f = |o: u64| emu.read_f32(rec + o);
            bin.push(PlaneRecord {
                plane: [f(0x0), f(0x4), f(0x8), f(0xc)],
                v0: [f(0x10), f(0x14), f(0x18)],
                v1: [f(0x1c), f(0x20), f(0x24)],
                v2: [f(0x28), f(0x2c), f(0x30)],
            });
        }
        // the +0x18 array and its count at +0x14
        let bin_rowcount = emu.read_u32(facet_arr + 0x14);
        let bin_rows: Vec<[f32; 2]> = (0..n as u64)
            .map(|r| {
                [
                    emu.read_f32(facet_aux + r * 8),
                    emu.read_f32(facet_aux + r * 8 + 4),
                ]
            })
            .collect();

        let chunk = ChunkModel {
            flags: 0,
            origin,
            normals_off: chunk_base as u32 + 0x170,
            verts: &verts,
            holes: *holes,
        };
        let mut ours = FacetArray::default();
        chunk_gather_aabb(&chunk, aabb, *rect, &mut ours, 0);

        assert_eq!(
            bin.len(),
            ours.records.len(),
            "gather_aabb nonzero-origin case#{ci}: record count"
        );
        // grown to the count and zeroed here too
        assert_eq!(
            bin_rowcount, n,
            "nz case#{ci}: rows count (+0x14) == record count"
        );
        assert_eq!(bin_rows.len(), ours.rows.len(), "nz case#{ci}: rows len");
        for (i, (b, o)) in bin_rows.iter().zip(ours.rows.iter()).enumerate() {
            assert_f32(b[0], o[0], &format!("nz case#{ci} row[{i}].x"));
            assert_f32(b[1], o[1], &format!("nz case#{ci} row[{i}].y"));
        }
        // case 0 covers the whole chunk, so it must keep triangles
        if ci == 0 {
            assert!(
                !bin.is_empty(),
                "nonzero-origin case#0: binary gathered no triangles (box should cover all)"
            );
        }
        for (i, (b, o)) in bin.iter().zip(ours.records.iter()).enumerate() {
            for k in 0..4 {
                assert_f32(
                    b.plane[k],
                    o.plane[k],
                    &format!("nz case#{ci} rec[{i}].plane[{k}]"),
                );
            }
            for k in 0..3 {
                assert_f32(b.v0[k], o.v0[k], &format!("nz case#{ci} rec[{i}].v0[{k}]"));
                assert_f32(b.v1[k], o.v1[k], &format!("nz case#{ci} rec[{i}].v1[{k}]"));
                assert_f32(b.v2[k], o.v2[k], &format!("nz case#{ci} rec[{i}].v2[{k}]"));
            }
        }
    }
}

#[test]
fn chunk_gather_planes_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let chunk_base = emu.alloc(0x6000);
    let tile_base = emu.alloc(0x2000);
    // The volume: the function copies 0x18 dwords twice and more; the planes at +0x0, the position
    // at +0x6c set to 0. Generous and cleared.
    let vol_p = emu.alloc(0x200);
    let rect_p = emu.alloc(0x20);
    let facet_arr = emu.alloc(0x40);
    let facet_store = emu.alloc(0x34 * 0x400);

    emu.write_u32(DAT_C9E388, 0);
    emu.write_u32(DAT_C7B2A4, 0);

    // six planes around the chunk, relative to it: inside when n·p + d >= −ε
    let big = 1e9f32;
    let planes: [[f32; 4]; 6] = [
        [1.0, 0.0, 0.0, big],
        [-1.0, 0.0, 0.0, big],
        [0.0, 1.0, 0.0, big],
        [0.0, -1.0, 0.0, big],
        [0.0, 0.0, 1.0, big],
        [0.0, 0.0, -1.0, big],
    ];
    let tight: [[f32; 4]; 6] = [
        [1.0, 0.0, 0.0, big],
        [-1.0, 0.0, 0.0, big],
        [0.0, 1.0, 0.0, big],
        [0.0, -1.0, 0.0, big],
        [0.0, 0.0, 1.0, -100.0], // z >= 100
        [0.0, 0.0, -1.0, 110.0], // z <= 110
    ];

    let mut rng = Lcg(0x9e37_79b9);
    let cases: Vec<(u16, [i32; 4], [[f32; 4]; 6])> = vec![
        (0x0000, [0, 0, 7, 7], planes),
        (0x0002, [0, 0, 7, 7], planes),
        (0x0000, [0, 0, 7, 7], tight),
        (0x0000, [-1, -3, 8, 11], planes),
    ];

    let mut emit_cases = 0u32;
    let mut skip_cases = 0u32;
    let mut hole_cases = 0u32;
    for (ci, (holes, rect, planeset)) in cases.iter().enumerate() {
        let verts = write_chunk(&mut emu, chunk_base, *holes, 0x0, &mut rng);
        wire_area_table(&mut emu, tile_base, chunk_base);
        // the volume cleared, the planes at +0x0, the position at +0x6c 0
        for b in 0..0x200u64 / 4 {
            emu.write_u32(vol_p + 4 * b, 0);
        }
        for (pi, pl) in planeset.iter().enumerate() {
            for (k, &c) in pl.iter().enumerate() {
                emu.write_u32(vol_p + 0x10 * pi as u64 + 4 * k as u64, c.to_bits());
            }
        }
        emu.write_u32(facet_arr, 0x400);
        emu.write_u32(facet_arr + 0x4, 0);
        emu.write_u32(facet_arr + 0x8, facet_store as u32);
        for o in (0xc..0x20u64).step_by(4) {
            emu.write_u32(facet_arr + o, 0);
        }
        for b in 0..(0x34 * 0x400u64) / 4 {
            emu.write_u32(facet_store + 4 * b, 0);
        }
        write_i32s(&mut emu, rect_p, rect);
        // ecx = 0, edx = 0, then &rect, &volume, &list and the mask 0xf00
        emu.call(
            0x006a_c1e0,
            0,
            0,
            &[rect_p as u32, vol_p as u32, facet_arr as u32, 0xf00],
        );
        let n = emu.read_u32(facet_arr + 0x4);
        let mut bin = Vec::new();
        for r in 0..n as u64 {
            let rec = facet_store + r * 0x34;
            let f = |o: u64| emu.read_f32(rec + o);
            bin.push(PlaneRecord {
                plane: [f(0x0), f(0x4), f(0x8), f(0xc)],
                v0: [f(0x10), f(0x14), f(0x18)],
                v1: [f(0x1c), f(0x20), f(0x24)],
                v2: [f(0x28), f(0x2c), f(0x30)],
            });
        }

        let chunk = ChunkModel {
            flags: 0,
            origin: [0.0, 0.0, 0.0],
            normals_off: chunk_base as u32 + 0x170,
            verts: &verts,
            holes: *holes,
        };
        let mut ours = FacetArray::default();
        chunk_gather_planes(&chunk, planeset, *rect, &mut ours);

        assert_eq!(
            bin.len(),
            ours.records.len(),
            "gather_planes case#{ci}: record count"
        );
        for (i, (b, o)) in bin.iter().zip(ours.records.iter()).enumerate() {
            for k in 0..4 {
                assert_f32(
                    b.plane[k],
                    o.plane[k],
                    &format!("case#{ci} rec[{i}].plane[{k}]"),
                );
            }
            for k in 0..3 {
                assert_f32(b.v0[k], o.v0[k], &format!("case#{ci} rec[{i}].v0[{k}]"));
                assert_f32(b.v1[k], o.v1[k], &format!("case#{ci} rec[{i}].v1[{k}]"));
                assert_f32(b.v2[k], o.v2[k], &format!("case#{ci} rec[{i}].v2[{k}]"));
            }
        }
        if !bin.is_empty() {
            emit_cases += 1;
        }
        if bin.len() < 4 * 64 {
            skip_cases += 1;
        }
        if *holes != 0 {
            hole_cases += 1;
        }
    }
    assert!(emit_cases > 0 && skip_cases > 0 && hole_cases > 0);
}
