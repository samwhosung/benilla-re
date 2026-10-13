//! The fills, each run whole: `CChunkLiquid::FillRiverVertices` (`0x68d790`), `FillOceanVertices`
//! (`0x68d690`) and `FillMagmaVertices` (`0x68d890`); `InitTerrainDriftConstants` (`0x68ebf0`);
//! `BuildChunkMeshTables` (`0x6aef50`); `CMapAreaLow::FillVertices` (`0x6bd8c0`) with the buffer
//! lock, unlock and release (`0x58a080`, `0x58a0a0`, `0x58a030`) stubbed; `BuildChunkIndices`
//! (`0x6c51b0`); and `IntersectCellRects` (`0x6adbf0`).

use benilla_re_difftest::{Emu, RegisterX86, Wow};

const FX_TABLE_VA: u64 = 0x00c8_1b98; // 0x68ebf0's table

const LUT_C81768_VA: u64 = 0x00c8_1768; // the river fill's colours

const LUT_C7FCD8_VA: u64 = 0x00c7_fcd8; // the ocean fill's colours

const QUAD_CURSOR_VA: u64 = 0x00ca_c3ac; // 0x6c51b0's cursor, a pointer

const MESH_UV_VA: u64 = 0x00ca_c860; // 0x6aef50's scaled table

const MESH_POS_VA: u64 = 0x00ca_c3c8; // 0x6aef50's unscaled table

fn lcg(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    }
}

fn assert_u32_slice(binary: &[u32], ours: &[u32], ctx: &str) {
    assert_eq!(binary.len(), ours.len(), "{ctx}: length");
    for i in 0..binary.len() {
        assert_eq!(
            binary[i], ours[i],
            "{ctx}[{i}]: binary={:#010x} ours={:#010x}",
            binary[i], ours[i]
        );
    }
}

fn assert_f32_slice(binary: &[f32], ours: &[f32], ctx: &str) {
    assert_eq!(binary.len(), ours.len(), "{ctx}: length");
    for i in 0..binary.len() {
        let (b, o) = (binary[i], ours[i]);
        if b.is_nan() && o.is_nan() {
            continue;
        }
        assert_eq!(
            b.to_bits(),
            o.to_bits(),
            "{ctx}[{i}]: binary={:#010x} ours={:#010x}",
            b.to_bits(),
            o.to_bits()
        );
    }
}

/// The river and ocean fills share one liquid and one stream of 60 cases from the same seed;
/// `mclq_env` writes the colour tables at their addresses, `mclq_case` a case's liquid.
struct MclqEnv<'a> {
    emu: Emu<'a>,
    layer: u64,
    depth_buf: u64,
    out: u64,
    lut_a_vals: [u32; 256],
    lut_b_vals: [u32; 256],
}

fn mclq_env<'a>(wow: &Wow) -> (MclqEnv<'a>, impl FnMut() -> u32) {
    let mut emu = Emu::new(wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let layer = emu.alloc(0x34 + 81 * 0xc + 0x10); // the positions inline at +0x34, 81 × 3 floats
    let depth_buf = emu.alloc(81 * 8 + 8); // +0xc points here, a depth byte every 8
    let out = emu.alloc(81 * 10 * 4);
    let lut_a = emu.alloc(256 * 4);
    let lut_b = emu.alloc(256 * 4);
    // the colour tables at their own addresses, which the function reads
    let mut rng = lcg(0xc010_a7e5);
    let mut lut_a_vals = [0u32; 256];
    let mut lut_b_vals = [0u32; 256];
    for i in 0..256 {
        lut_a_vals[i] = rng();
        lut_b_vals[i] = rng();
        emu.write_u32(LUT_C81768_VA + i as u64 * 4, lut_a_vals[i]);
        emu.write_u32(LUT_C7FCD8_VA + i as u64 * 4, lut_b_vals[i]);
    }
    // a copy in the arena too, which the function does not read
    for i in 0..256 {
        emu.write_u32(lut_a + i as u64 * 4, lut_a_vals[i]);
        emu.write_u32(lut_b + i as u64 * 4, lut_b_vals[i]);
    }
    (
        MclqEnv {
            emu,
            layer,
            depth_buf,
            out,
            lut_a_vals,
            lut_b_vals,
        },
        rng,
    )
}

/// Writes a case's positions and depths, and returns them with whether the extremes occurred.
fn mclq_case(
    env: &mut MclqEnv<'_>,
    rng: &mut impl FnMut() -> u32,
    case: u32,
) -> ([[u32; 3]; 81], [u8; 81], u32, u32) {
    let mut pos = [[0u32; 3]; 81];
    for (k, p) in pos.iter_mut().enumerate() {
        for (c, slot) in p.iter_mut().enumerate() {
            let bits = rng();
            *slot = bits;
            env.emu
                .write_u32(env.layer + 0x34 + (k * 3 + c) as u64 * 4, bits);
        }
    }
    let mut depth = [0u8; 81];
    let (mut saw_zero, mut saw_max) = (0u32, 0u32);
    for (k, d) in depth.iter_mut().enumerate() {
        let b = match (case, k) {
            (0, _) => 0,   // every depth 0
            (1, _) => 255, // every depth 255
            _ => (rng() >> 13) as u8,
        };
        *d = b;
        if b == 0 {
            saw_zero += 1;
        }
        if b == 255 {
            saw_max += 1;
        }
        env.emu.write(env.depth_buf + k as u64 * 8, &[b]);
    }
    env.emu.write_u32(env.layer + 0xc, env.depth_buf as u32);
    (pos, depth, saw_zero, saw_max)
}

fn mclq_read_out(emu: &Emu, out: u64) -> Vec<u32> {
    (0..81 * 10)
        .map(|i| emu.read_u32(out + i as u64 * 4))
        .collect()
}

/// `0x68d790` with the table at `0xc81768`; depths 0 and 255 must occur. 81 vertices of 10 dwords.
#[test]
fn mclq_verts_a() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut env, mut rng) = mclq_env(&wow);
    let (mut saw_zero, mut saw_max, mut checked) = (0u32, 0u32, 0u32);
    for case in 0..60u32 {
        let (pos, depth, z, m) = mclq_case(&mut env, &mut rng, case);
        saw_zero += z;
        saw_max += m;
        for i in 0..81 * 10 {
            env.emu.write_u32(env.out + i as u64 * 4, 0xcccc_cccc);
        }
        env.emu
            .call(0x68d790, env.layer as u32, 0, &[env.out as u32]);
        let ours_a = benilla_re_terrain::fills::mclq_verts_a(&pos, &depth, &env.lut_a_vals);
        let flat_a: Vec<u32> = ours_a.iter().flatten().copied().collect();
        assert_u32_slice(
            &mclq_read_out(&env.emu, env.out),
            &flat_a,
            &format!("mclq_verts_a case {case}"),
        );
        checked += 1;
    }
    assert!(saw_zero > 0, "depth-byte 0 extreme never hit");
    assert!(saw_max > 0, "depth-byte 255 extreme never hit");
    eprintln!(
        "mclq_verts_a: {checked} fills bit-exact vs WoW.exe @ 0x68d790 (depth 0×{saw_zero} 255×{saw_max})"
    );
}

/// `0x68d690` with the table at `0xc7fcd8`, on the same cases.
#[test]
fn mclq_verts_b() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut env, mut rng) = mclq_env(&wow);
    let (mut saw_zero, mut saw_max, mut checked) = (0u32, 0u32, 0u32);
    for case in 0..60u32 {
        let (pos, depth, z, m) = mclq_case(&mut env, &mut rng, case);
        saw_zero += z;
        saw_max += m;
        for i in 0..81 * 10 {
            env.emu.write_u32(env.out + i as u64 * 4, 0xcccc_cccc);
        }
        env.emu
            .call(0x68d690, env.layer as u32, 0, &[env.out as u32]);
        let ours_b = benilla_re_terrain::fills::mclq_verts_b(&pos, &depth, &env.lut_b_vals);
        let flat_b: Vec<u32> = ours_b.iter().flatten().copied().collect();
        assert_u32_slice(
            &mclq_read_out(&env.emu, env.out),
            &flat_b,
            &format!("mclq_verts_b case {case}"),
        );
        checked += 1;
    }
    assert!(saw_zero > 0, "depth-byte 0 extreme never hit");
    assert!(saw_max > 0, "depth-byte 255 extreme never hit");
    eprintln!(
        "mclq_verts_b: {checked} fills bit-exact vs WoW.exe @ 0x68d690 (depth 0×{saw_zero} 255×{saw_max})"
    );
}

/// `0x68d890`: the texture words at +0xc, 8 apart, rows 0x48 apart; words 0 and 0xffff must occur.
#[test]
fn liquid_render_verts_fill() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let layer = emu.alloc(0x34 + 81 * 0xc + 0x10);
    let tc_buf = emu.alloc(9 * 0x48 + 0x10); // +0xc points here, rows 0x48 apart
    let out = emu.alloc(81 * 10 * 4);
    let mut rng = lcg(0x011d_ce11);
    let mut saw0 = 0u32;
    let mut sawmax = 0u32;
    let mut checked = 0u32;
    for case in 0..60u32 {
        let mut pos = [[0u32; 3]; 81];
        for (k, p) in pos.iter_mut().enumerate() {
            for (c, slot) in p.iter_mut().enumerate() {
                let bits = rng();
                *slot = bits;
                emu.write_u32(layer + 0x34 + (k * 3 + c) as u64 * 4, bits);
            }
        }
        let mut tc = [[0u16; 2]; 81];
        for (k, slot) in tc.iter_mut().enumerate() {
            let row = k / 9;
            let col = k % 9;
            let (u, v) = match case {
                0 => (0u16, 0u16),
                1 => (0xffff, 0xffff),
                _ => ((rng() >> 14) as u16, (rng() >> 14) as u16),
            };
            *slot = [u, v];
            if u == 0 || v == 0 {
                saw0 += 1;
            }
            if u == 0xffff || v == 0xffff {
                sawmax += 1;
            }
            let off = tc_buf + (row * 0x48 + col * 8) as u64;
            emu.write_u32(off, u32::from(u));
            emu.write_u32(off + 2, u32::from(v));
        }
        emu.write_u32(layer + 0xc, tc_buf as u32);
        for i in 0..81 * 10 {
            emu.write_u32(out + i as u64 * 4, 0xcccc_cccc);
        }
        emu.call(0x68d890, layer as u32, 0, &[out as u32]);
        let got: Vec<u32> = (0..81 * 10)
            .map(|i| emu.read_u32(out + i as u64 * 4))
            .collect();
        let ours = benilla_re_terrain::fills::liquid_render_verts(&pos, &tc);
        let flat: Vec<u32> = ours.iter().flatten().copied().collect();
        assert_u32_slice(&got, &flat, &format!("liquid_render_verts case {case}"));
        checked += 1;
    }
    assert!(saw0 > 0 && sawmax > 0, "u16 extremes not exercised");
    eprintln!("liquid_render_verts_fill: {checked} fills bit-exact vs WoW.exe @ 0x68d890");
}

/// `0x68ebf0`'s whole table at `0xc81b98`; it has no inputs.
#[test]
fn fx_uv_table() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    // the region filled first, so a missing store shows
    for i in 0..104u64 {
        emu.write_u32(FX_TABLE_VA + i * 4, 0xdead_beef);
    }
    emu.call(0x68ebf0, 0, 0, &[]);
    let got: Vec<f32> = (0..104)
        .map(|i| emu.read_f32(FX_TABLE_VA + i as u64 * 4))
        .collect();
    let ours = benilla_re_terrain::fills::fx_uv_table();
    assert_f32_slice(&got, &ours, "fx_uv_table");
    eprintln!("fx_uv_table: 26×4 table bit-exact vs WoW.exe @ 0x68ebf0");
}

/// `0x6aef50`'s two tables, 145 pairs each; the copies per level after them read pointer tables at
/// `0x86b540` that are 0 in the image, so they copy nothing. It has no inputs.
#[test]
fn chunk_mesh_tables() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    for i in 0..145u64 * 2 {
        emu.write_u32(MESH_UV_VA + i * 4, 0xdead_beef);
        emu.write_u32(MESH_POS_VA + i * 4, 0xdead_beef);
    }
    emu.call(0x6aef50, 0, 0, &[]);
    let read = |emu: &Emu, base: u64| -> Vec<f32> {
        (0..145 * 2)
            .map(|i| emu.read_f32(base + i as u64 * 4))
            .collect()
    };
    let ours = benilla_re_terrain::fills::chunk_mesh_tables();
    let our_uv: Vec<f32> = ours.uv.iter().flatten().copied().collect();
    let our_pos: Vec<f32> = ours.pos.iter().flatten().copied().collect();
    assert_f32_slice(&read(&emu, MESH_UV_VA), &our_uv, "chunk_mesh uv");
    assert_f32_slice(&read(&emu, MESH_POS_VA), &our_pos, "chunk_mesh pos");
    eprintln!("chunk_mesh_tables: 145 UV + 145 pos pairs bit-exact vs WoW.exe @ 0x6aef50");
}

/// `0x6bd8c0` (ecx = this): the buffer's lock hands out the test's buffer. A range of starting
/// corners and random heights.
#[test]
fn wdl_mesh_fill() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let nverts = 289 + 256;
    let buf = emu.alloc(nverts as u64 * 16 + 0x20);
    emu.stub(0x58a080, buf as u32, 0); // the lock returns the test's buffer
    emu.stub(0x58a0a0, 0, 0); // the unlock
    emu.stub(0x58a030, 0, 0); // the release
    let this = emu.alloc(0x4c0 + 256 * 4 + 0x10);
    let mut rng = lcg(0x3d10_ee44);
    let seeds: [(f32, f32); 6] = [
        (0.0, 0.0),
        (17066.0, 17066.0),
        (-17066.0, -17066.0),
        (533.5, -533.5),
        (1.0, -1.0),
        (33.33333, -16.66666),
    ];
    let mut checked = 0u32;
    for case in 0..40u32 {
        let (x0, y0) = seeds[case as usize % seeds.len()];
        let mut outer = [0f32; 289];
        let mut inner = [0f32; 256];
        for (i, h) in outer.iter_mut().enumerate() {
            *h = if case == 0 {
                i as f32
            } else {
                f32::from_bits(rng())
            };
            // no NaN heights, though they are copied as dwords
            if h.is_nan() {
                *h = i as f32 * 0.5;
            }
            emu.write_u32(this + 0x3c + i as u64 * 4, h.to_bits());
        }
        for (i, h) in inner.iter_mut().enumerate() {
            *h = if case == 0 {
                -(i as f32)
            } else {
                f32::from_bits(rng())
            };
            if h.is_nan() {
                *h = i as f32 * -0.5;
            }
            emu.write_u32(this + 0x4c0 + i as u64 * 4, h.to_bits());
        }
        emu.write_u32(this + 0x28, x0.to_bits());
        emu.write_u32(this + 0x2c, y0.to_bits());
        for i in 0..nverts as u64 * 4 {
            emu.write_u32(buf + i * 4, 0xeeee_eeee);
        }
        emu.call(0x6bd8c0, this as u32, 0, &[]);
        let got: Vec<u32> = (0..nverts as u64 * 4)
            .map(|i| emu.read_u32(buf + i * 4))
            .collect();
        let ours = benilla_re_terrain::fills::wdl_mesh_fill(x0, y0, &outer, &inner);
        let flat: Vec<u32> = ours.iter().flatten().copied().collect();
        assert_u32_slice(&got, &flat, &format!("wdl_mesh_fill case {case}"));
        checked += 1;
    }
    eprintln!("wdl_mesh_fill: {checked} fills bit-exact vs WoW.exe @ 0x6bd8c0");
}

/// `0x6c51b0` (ecx = level, edx = cap, then caps, holes, x, y): the holes at level 1, the
/// subdivision below `cap`, and the leaf with each edge fan split, each counted and required. The
/// cursor `0xcac3ac` points at the test's buffer; the indices and the final cursor are compared.
#[test]
fn quad_hole_strip_indices() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let buf = emu.alloc(0x4000);
    let mut rng = lcg(0x0bad_f00d);
    // the paths taken
    let mut hit_hole = 0u32;
    let mut hit_subdiv = 0u32;
    let mut hit_leaf = 0u32;
    let mut hit_edges = [0u32; 4];
    let mut checked = 0u32;

    // Fixed cases for each path first, then random ones. With holes at level 1 the table index is
    // `((x' − 1) >> 1) + 4·((y' − 1) >> 1)`; the caller's coordinates are at least 1, so these cases
    // use x and y of at least 2.
    let mut cases: Vec<(u32, u32, u32, u32, i32, i32)> = vec![
        // (level, cap, caps, holes, x, y)
        (1, 0, 0x0000_0000, 0x0000_1111, 2, 2), // holes at level 1
        (1, 0, 0x0000_0000, 0x0000_ffff, 4, 6), // holes at level 1, every bit set
        (3, 3, 0x0403_0201, 0, 5, 7),           // a leaf at level 3
        (0, 3, 0x0001_0203, 0, 0, 0),           // subdivision
        (2, 2, 0xffff_ffff, 0, 1, 1),           // a leaf with every fan split
        (3, 3, 0x0000_0000, 0, 0, 0),           // a leaf with none split
        (3, 3, 0x0404_0404, 0, -3, -2),         // a leaf, negative coordinates
    ];
    for _ in 0..400 {
        let level = rng() % 4;
        let cap = rng() % 4;
        let caps = ((rng() % 5) << 24) | ((rng() % 5) << 16) | ((rng() % 5) << 8) | (rng() % 5);
        // A hole word only with level 1 and coordinates of at least 2, as the caller passes; the
        // other paths do not read it.
        let hole_path = level == 1 && (rng() >> 8).is_multiple_of(2);
        let (gx, gy) = if hole_path {
            ((2 + rng() % 7) as i32, (2 + rng() % 7) as i32)
        } else {
            ((rng() % 9) as i32 - 2, (rng() % 9) as i32 - 2)
        };
        let holes = if hole_path { rng() & 0xffff } else { 0 };
        cases.push((level, cap, caps, holes, gx, gy));
    }

    for (level, cap, caps, holes, gx, gy) in cases {
        // the path, as the reimplementation takes it
        if holes != 0 && level == 1 {
            hit_hole += 1;
        } else if (level as i32) < (cap as i32) {
            hit_subdiv += 1;
        } else {
            hit_leaf += 1;
            if level < (caps >> 0x18) {
                hit_edges[0] += 1;
            }
            if level < ((caps >> 0x10) & 0xff) {
                hit_edges[1] += 1;
            }
            if level < ((caps >> 8) & 0xff) {
                hit_edges[2] += 1;
            }
            if level < (caps & 0xff) {
                hit_edges[3] += 1;
            }
        }

        // the buffer filled first, so a short run shows
        for i in 0..0x800u64 {
            emu.write_u32(buf + i * 2, 0x7777);
        }
        emu.write_u32(QUAD_CURSOR_VA, buf as u32);
        emu.call(0x6c51b0, level, cap, &[caps, holes, gx as u32, gy as u32]);
        let end = u64::from(emu.read_u32(QUAD_CURSOR_VA));

        let mut ours = Vec::new();
        benilla_re_terrain::fills::quad_hole_strip_indices(
            level, cap, caps, holes, gx, gy, &mut ours,
        );

        // The cursor ends 2 bytes on per value, the first written in place and a last advance
        // after the end.
        let got: Vec<i16> = (0..ours.len())
            .map(|i| emu.read_u32(buf + i as u64 * 2) as i16)
            .collect();
        assert_eq!(
            got, ours,
            "quad emit (level={level} cap={cap} caps={caps:#010x} holes={holes:#x} gx={gx} gy={gy})"
        );
        assert_eq!(
            end,
            buf + 2 * ours.len() as u64,
            "quad cursor (level={level} caps={caps:#010x} holes={holes:#x} N={})",
            ours.len()
        );
        checked += 1;
    }
    assert!(hit_hole > 0, "hole-recursion regime never hit");
    assert!(hit_subdiv > 0, "subdivision regime never hit");
    assert!(hit_leaf > 0, "leaf regime never hit");
    for (i, &c) in hit_edges.iter().enumerate() {
        assert!(c > 0, "leaf edge-fan {i} never gated wide");
    }
    eprintln!(
        "quad_hole_strip_indices: {checked} emits bit-exact vs WoW.exe @ 0x6c51b0 \
         (hole×{hit_hole} subdiv×{hit_subdiv} leaf×{hit_leaf} edges={hit_edges:?})"
    );
}

/// `0x6adbf0` (ecx = out, edx = b, c on the stack) over every order of a small domain, ties and
/// empty results included.
#[test]
fn aabb_intersect_i() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let out = emu.alloc(0x10);
    let b = emu.alloc(0x10);
    let c = emu.alloc(0x10);
    let mut rng = lcg(0xaabb_0011);
    let small = [-3i32, -1, 0, 1, 3, i32::MIN, i32::MAX];
    let mut cases: Vec<([i32; 4], [i32; 4])> = Vec::new();
    // every tie over a small domain
    for &bx in &small {
        for &cx in &small {
            cases.push(([bx, bx, bx, bx], [cx, cx, cx, cx]));
        }
    }
    // empty results and random orders
    cases.push(([10, 10, 0, 0], [0, 0, 10, 10])); // b inverted
    cases.push(([0, 0, 5, 5], [6, 6, 9, 9])); // disjoint: empty
    for _ in 0..400 {
        let mk = |rng: &mut dyn FnMut() -> u32| {
            [
                (rng() & 0x3f) as i32 - 32,
                (rng() & 0x3f) as i32 - 32,
                (rng() & 0x3f) as i32 - 32,
                (rng() & 0x3f) as i32 - 32,
            ]
        };
        cases.push((mk(&mut rng), mk(&mut rng)));
    }

    let mut empty_seen = 0u32;
    let mut checked = 0u32;
    for (bv, cv) in cases {
        for i in 0..4 {
            emu.write_u32(b + i as u64 * 4, bv[i] as u32);
            emu.write_u32(c + i as u64 * 4, cv[i] as u32);
            emu.write_u32(out + i as u64 * 4, 0xdead_beef);
        }
        emu.call(0x6adbf0, out as u32, b as u32, &[c as u32]);
        let got = [
            emu.read_u32(out) as i32,
            emu.read_u32(out + 4) as i32,
            emu.read_u32(out + 8) as i32,
            emu.read_u32(out + 0xc) as i32,
        ];
        let ours = benilla_re_terrain::fills::aabb_intersect_i(bv, cv);
        assert_eq!(got, ours, "aabb_intersect_i (b={bv:?} c={cv:?})");
        if ours[0] > ours[2] || ours[1] > ours[3] {
            empty_seen += 1;
        }
        checked += 1;
    }
    assert!(empty_seen > 0, "no empty intersections exercised");
    eprintln!(
        "aabb_intersect_i: {checked} merges bit-exact vs WoW.exe @ 0x6adbf0 (empty×{empty_seen})"
    );
}
