//! The facet emitters, each run whole: `AddQuadFacets` (`0x6aba30`), `CMapChunk::AddImpassableWalls`
//! (`0x6ab530`) and `FlushLiquidFacets` (`0x671cc0`) on its x87 path, each into a list sized so
//! its growers do not reallocate.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::collide::{
    impassable_chunk_fence, liquid_hit_flush, quad_collision_emit, ChunkBounds, Facet, LiquidBatch,
};
use common::assert_f32;

const RECORD_STRIDE: u64 = 0x34;

/// Compares a facet in the arena with the reimplementation's, field by field.
fn assert_facet(emu: &Emu, slot_va: u64, ours: &Facet, ctx: &str) {
    for (k, &o) in ours.iter().enumerate() {
        let b = emu.read_f32(slot_va + k as u64 * 4);
        assert_f32(b, o, &format!("{ctx}.f{k}"));
    }
}

/// Makes an empty facet list at `hdr`: capacity `cap`, count 0, buffer `buf`.
fn init_list(emu: &mut Emu, hdr: u64, buf: u64, cap: u32) {
    emu.write_u32(hdr, cap); // capacity
    emu.write_u32(hdr + 4, 0); // count
    emu.write_u32(hdr + 8, buf as u32); // buffer
    emu.write_u32(hdr + 0xc, 0); // alignment
}

/// The address of facet `i` in the buffer.
fn slot(buf: u64, i: usize) -> u64 {
    buf + i as u64 * RECORD_STRIDE
}

/// Random coordinates within `[-2000, 2000]` and small exact edges: the world's magnitudes. With
/// magnitudes near `f32`'s limit the products overflow, where the reimplementation does not match
/// the exe; no caller passes such values.
fn synth_coord(s: &mut u32) -> f32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    if (*s >> 8) & 0xf == 0 {
        let edges = [
            0.0f32, -0.0, 1.0, -1.0, 0.5, 2.0, 8.0, 16.0, 33.333, -33.333, 100.0, 0.25, 0.03125,
            -0.5,
        ];
        edges[((*s >> 12) as usize) % edges.len()]
    } else {
        let u = (*s >> 8) as f32 / (1u32 << 24) as f32; // [0, 1)
        u * 4000.0 - 2000.0
    }
}

fn synth_vec3(s: &mut u32) -> [f32; 3] {
    [synth_coord(s), synth_coord(s), synth_coord(s)]
}

fn write_vec3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, &c) in v.iter().enumerate() {
        emu.write_u32(va + i as u64 * 4, c.to_bits());
    }
}

/// Write a `ChunkBounds` block at `chunk+0x44 .. +0x54`.
fn write_bounds(emu: &mut Emu, chunk: u64, b: &ChunkBounds) {
    emu.write_u32(chunk + 0x44, b.min_x.to_bits());
    emu.write_u32(chunk + 0x48, b.min_y.to_bits());
    emu.write_u32(chunk + 0x4c, b.z.to_bits());
    emu.write_u32(chunk + 0x50, b.max_x.to_bits());
    emu.write_u32(chunk + 0x54, b.max_y.to_bits());
}

/// Write a 4×4 column-major matrix at `va`.
fn write_mat4(emu: &mut Emu, va: u64, m: &[f32; 16]) {
    for (i, &c) in m.iter().enumerate() {
        emu.write_u32(va + i as u64 * 4, c.to_bits());
    }
}

/// Write a flat f32 pool (vec3s laid out stride-12).
fn write_vec3_pool(emu: &mut Emu, base: u64, data: &[f32]) {
    for (i, &c) in data.iter().enumerate() {
        emu.write_u32(base + i as u64 * 4, c.to_bits());
    }
}

/// Write a u16 pool.
fn write_u16_pool(emu: &mut Emu, base: u64, data: &[u16]) {
    for (i, &v) in data.iter().enumerate() {
        emu.write(base + i as u64 * 2, &v.to_le_bytes());
    }
}

#[test]
fn quad_collision_emit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);

    let hdr = emu.alloc(0x10);
    let buf = emu.alloc(0x4000); // room for many facets
    let p = emu.alloc(0x10);
    let n = emu.alloc(0x10);
    let e0 = emu.alloc(0x10);
    let e1 = emu.alloc(0x10);

    let mut s: u32 = 0x1234_abcd;
    let mut checked = 0u32;
    for _ in 0..3000 {
        // a fresh list each case, so the facets are at slots 0 and 1
        init_list(&mut emu, hdr, buf, 0x1000);
        let (pv, nv, e0v, e1v) = (
            synth_vec3(&mut s),
            synth_vec3(&mut s),
            synth_vec3(&mut s),
            synth_vec3(&mut s),
        );
        write_vec3(&mut emu, p, &pv);
        write_vec3(&mut emu, n, &nv);
        write_vec3(&mut emu, e0, &e0v);
        write_vec3(&mut emu, e1, &e1v);

        // ecx = list, edx = p, n, e0 and e1 on the stack, `ret 0xc`
        emu.call(
            0x6aba30,
            hdr as u32,
            p as u32,
            &[n as u32, e0 as u32, e1 as u32],
        );

        let mut ours: Vec<Facet> = Vec::new();
        quad_collision_emit(&mut ours, &pv, &nv, &e0v, &e1v);
        assert_eq!(emu.read_u32(hdr + 4), 2, "quad appends 2 records");
        assert_eq!(ours.len(), 2);
        for (i, o) in ours.iter().enumerate() {
            assert_facet(&emu, slot(buf, i), o, &format!("quad case rec{i}"));
        }
        checked += 1;
    }
    eprintln!("quad_collision_emit: {checked} quads (2 recs each) bit-exact vs WoW.exe @ 0x6aba30");
}

#[test]
fn impassable_chunk_fence_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);

    let hdr = emu.alloc(0x10);
    let buf = emu.alloc(0x4000);
    let chunk = emu.alloc(0x80);
    let pbox = emu.alloc(0x20); // the AABB p: 6 f32

    // Each side's crossing is counted and must happen, and a box wholly inside covers every skip.
    let mut side_hits = [0u32; 4];
    let mut all_inside = 0u32;
    let mut checked = 0u32;

    let mut s: u32 = 0x55aa_1234;
    // Fixed cases first: a box out of exactly one side, then one wholly inside.
    let base_bounds = ChunkBounds {
        min_x: -50.0,
        min_y: -40.0,
        z: 12.0,
        max_x: 50.0,
        max_y: 40.0,
    };
    // the four sides, then wholly inside
    let probe_boxes: [[f32; 6]; 5] = [
        [-10.0, -60.0, 0.0, 10.0, -45.0, 5.0], // side 1
        [-10.0, 45.0, 0.0, 10.0, 60.0, 5.0],   // side 2
        [-60.0, -10.0, 0.0, -45.0, 10.0, 5.0], // side 3
        [45.0, -10.0, 0.0, 60.0, 10.0, 5.0],   // side 4
        [-5.0, -5.0, 0.0, 5.0, 5.0, 5.0],      // no side
    ];

    for case in 0..3005usize {
        init_list(&mut emu, hdr, buf, 0x1000);

        let (bounds, pv): (ChunkBounds, [f32; 6]) = if let Some(pb) = probe_boxes.get(case) {
            (base_bounds, *pb)
        } else {
            // a chunk box, min below max, and a query box across it
            let cx = synth_coord(&mut s);
            let cy = synth_coord(&mut s);
            let hx = synth_coord(&mut s).abs() + 1.0;
            let hy = synth_coord(&mut s).abs() + 1.0;
            let b = ChunkBounds {
                min_x: (cx - hx).min(cx + hx),
                min_y: (cy - hy).min(cy + hy),
                z: synth_coord(&mut s),
                max_x: (cx - hx).max(cx + hx),
                max_y: (cy - hy).max(cy + hy),
            };
            let pmin = [
                synth_coord(&mut s),
                synth_coord(&mut s),
                synth_coord(&mut s),
            ];
            let pmax = [
                pmin[0] + synth_coord(&mut s).abs(),
                pmin[1] + synth_coord(&mut s).abs(),
                pmin[2] + synth_coord(&mut s).abs(),
            ];
            (b, [pmin[0], pmin[1], pmin[2], pmax[0], pmax[1], pmax[2]])
        };

        write_bounds(&mut emu, chunk, &bounds);
        for (i, &c) in pv.iter().enumerate() {
            emu.write_u32(pbox + i as u64 * 4, c.to_bits());
        }

        // which sides cross, counted
        if pv[1] < bounds.min_y {
            side_hits[0] += 1;
        }
        if bounds.max_y < pv[4] {
            side_hits[1] += 1;
        }
        if pv[0] < bounds.min_x {
            side_hits[2] += 1;
        }
        if bounds.max_x < pv[3] {
            side_hits[3] += 1;
        }
        if !(pv[1] < bounds.min_y
            || bounds.max_y < pv[4]
            || pv[0] < bounds.min_x
            || bounds.max_x < pv[3])
        {
            all_inside += 1;
        }

        // ecx = chunk, edx = &p, list on the stack, `ret 8`
        emu.call(0x6ab530, chunk as u32, pbox as u32, &[hdr as u32]);

        let mut ours: Vec<Facet> = Vec::new();
        impassable_chunk_fence(&mut ours, &bounds, &pv);

        let bin_count = emu.read_u32(hdr + 4) as usize;
        assert_eq!(
            bin_count,
            ours.len(),
            "case {case}: record count binary={bin_count} ours={}",
            ours.len()
        );
        for (i, o) in ours.iter().enumerate() {
            assert_facet(
                &emu,
                slot(buf, i),
                o,
                &format!("impassable_chunk_fence case {case} rec{i}"),
            );
        }
        checked += 1;
    }
    // every side crossed at least once, and the box wholly inside ran
    for (k, &h) in side_hits.iter().enumerate() {
        assert!(
            h > 0,
            "impassable_chunk_fence side {k} crossed branch never fired"
        );
    }
    assert!(
        all_inside > 0,
        "impassable_chunk_fence all-inside (no side) never fired"
    );
    eprintln!(
        "impassable_chunk_fence: {checked} cases bit-exact vs WoW.exe @ 0x6ab530 (side hits {side_hits:?}, all-inside {all_inside})"
    );
}

#[allow(clippy::too_many_lines)]
#[test]
fn liquid_hit_flush_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);

    // The globals the function reads: 0xc9e388, the SSE switch, 0 for the x87 path; 0xc5a474, the
    // entry count; 0xc62550, the entries, 0x20 bytes each.
    const GATE: u64 = 0x00c9_e388;
    const BATCH_COUNT: u64 = 0x00c5_a474;
    const BATCH_BUF: u64 = 0x00c6_2550;
    const BATCH_STRIDE: u64 = 0x20;

    // The list header is eight dwords: capacity, count, buffer, alignment, two watermarks, an array
    // the function writes an owner tag into per facet, and its alignment. The watermarks are set
    // high so it never reallocates.
    let hdr = emu.alloc(0x20);
    let buf = emu.alloc(0x8000);
    let out_arr = emu.alloc(0x4000); // the owner tags, 8 bytes per facet

    // the entries' storage, up to 4 entries of 8 triangles
    const MAX_ENTRIES: usize = 4;
    let mut mats = Vec::new();
    let mut vert_arenas = Vec::new();
    let mut idx_arenas = Vec::new();
    for _ in 0..MAX_ENTRIES {
        mats.push(emu.alloc(0x40)); // 16 floats
        vert_arenas.push(emu.alloc(0x100)); // up to about 21 vertices
        idx_arenas.push(emu.alloc(0x80)); // up to about 21 index triples
    }

    emu.write_u32(GATE, 0); // the x87 path

    let mut s: u32 = 0x0bad_f00d;
    let mut checked = 0u32;
    // the cases, each of which must occur
    let mut zero_entries = 0u32;
    let mut empty_entry = 0u32; // an entry with no triangles
    let mut single_tri = 0u32;
    let mut multi_tri = 0u32;
    let mut multi_entry = 0u32;
    let mut degenerate = 0u32; // a triangle with no area

    for case in 0..2000usize {
        init_list(&mut emu, hdr, buf, 0x2000);
        // the header's tail: high watermarks, the owner tags and their alignment
        emu.write_u32(hdr + 0x10, 0xffff_ffff);
        emu.write_u32(hdr + 0x14, 0xffff_ffff);
        emu.write_u32(hdr + 0x18, out_arr as u32);
        emu.write_u32(hdr + 0x1c, 0);

        // up to MAX_ENTRIES entries
        let n_entries = match case {
            0 => 0, // no entries
            1 => 1, // one entry with no triangles
            2 => 1, // one entry with one triangle
            3 => 1, // one entry with a triangle of no area
            _ => (case % MAX_ENTRIES) + 1,
        };
        if n_entries == 0 {
            zero_entries += 1;
        }
        if n_entries > 1 {
            multi_entry += 1;
        }

        let mut batches: Vec<LiquidBatch> = Vec::new();
        for e in 0..n_entries {
            // a matrix of entries near 1 and a translation, so the vertices stay finite
            let m: [f32; 16] = if case == 2 || case == 3 {
                [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 3.0, -2.0, 5.0, 1.0,
                ]
            } else {
                let mut mm = [0f32; 16];
                for c in &mut mm {
                    *c = synth_coord(&mut s) / 200.0; // entries near 1 to 10
                }
                mm[15] = 1.0;
                mm
            };
            write_mat4(&mut emu, mats[e], &m);

            // the entry's triangle count
            let tri_count: usize = match case {
                1 => 0,
                2 | 3 => 1,
                _ => 1 + ((s >> 9) as usize % 4),
            };
            if case != 1 {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            }
            if tri_count == 0 {
                empty_entry += 1;
            }
            if tri_count == 1 {
                single_tri += 1;
            }
            if tri_count > 1 {
                multi_tri += 1;
            }

            // three vertices per triangle
            let n_verts = (tri_count * 3).max(3);
            let mut verts = vec![0f32; n_verts * 3];
            if case == 3 {
                // three points on a line: the cross is 0 and the normalize divides by 0
                let a = [1.0, 2.0, 3.0];
                let b = [2.0, 4.0, 6.0];
                let c = [3.0, 6.0, 9.0];
                for (k, p) in [a, b, c].iter().enumerate() {
                    verts[k * 3] = p[0];
                    verts[k * 3 + 1] = p[1];
                    verts[k * 3 + 2] = p[2];
                }
                degenerate += 1;
            } else {
                for vc in verts.iter_mut() {
                    *vc = synth_coord(&mut s);
                }
            }
            write_vec3_pool(&mut emu, vert_arenas[e], &verts);

            // triangle t is (3t, 3t+1, 3t+2)
            let mut indices = vec![0u16; tri_count * 3];
            for (t, chunk) in indices.chunks_mut(3).enumerate() {
                chunk[0] = (3 * t) as u16;
                chunk[1] = (3 * t + 1) as u16;
                chunk[2] = (3 * t + 2) as u16;
            }
            write_u16_pool(&mut emu, idx_arenas[e], &indices);

            // entry e, its 0x20 bytes cleared first
            let entry = BATCH_BUF + e as u64 * BATCH_STRIDE;
            for w in 0..8u64 {
                emu.write_u32(entry + w * 4, 0);
            }
            emu.write_u32(entry, mats[e] as u32);
            emu.write_u32(entry + 0x04, vert_arenas[e] as u32);
            emu.write_u32(entry + 0x0c, idx_arenas[e] as u32);
            emu.write(entry + 0x16, &(tri_count as u16).to_le_bytes());

            batches.push(LiquidBatch {
                matrix: m,
                verts,
                indices,
                tri_count: tri_count as u16,
            });
        }
        emu.write_u32(BATCH_COUNT, n_entries as u32);

        // edx = list; the loop reads the globals
        emu.call(0x671cc0, 0, hdr as u32, &[0, 0, 0]);

        let mut ours: Vec<Facet> = Vec::new();
        liquid_hit_flush(&mut ours, &batches);

        let bin_count = emu.read_u32(hdr + 4) as usize;
        assert_eq!(
            bin_count,
            ours.len(),
            "case {case}: record count binary={bin_count} ours={}",
            ours.len()
        );
        for (i, o) in ours.iter().enumerate() {
            assert_facet(&emu, slot(buf, i), o, &format!("liquid case {case} rec{i}"));
        }
        checked += 1;
    }

    assert!(zero_entries > 0, "liquid: zero-entries case never ran");
    assert!(
        empty_entry > 0,
        "liquid: empty-entry (tri_count 0) never ran"
    );
    assert!(single_tri > 0, "liquid: single-tri never ran");
    assert!(multi_tri > 0, "liquid: multi-tri never ran");
    assert!(multi_entry > 0, "liquid: multi-entry never ran");
    assert!(degenerate > 0, "liquid: degenerate tri never ran");
    eprintln!(
        "liquid_hit_flush: {checked} cases bit-exact vs WoW.exe @ 0x671cc0 (x87 leg; \
         zero={zero_entries} empty={empty_entry} single={single_tri} multi={multi_tri} \
         multi_entry={multi_entry} degenerate={degenerate})"
    );
}
