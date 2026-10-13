//! The point queries, each run whole: `CrossVector3` (`0x672130`), `NormalizeVector3`
//! (`0x6720f0`), `CMapChunk::GetHeight` (`0x6b72a0`) on its x87 path, `CMapChunk::GetLiquidHeight`
//! (`0x6b7500`), `GetLiquidStatus` (`0x69b6d0`) with the map object query (`0x69b520`) stubbed to
//! find none, and `GetGroundEffect` (`0x69b1c0`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::assert_f32;

/// Random finite floats after a set of edge values; NaN is left out, as `fld`/`fstp` would quiet a
/// signalling one.
fn synth_f32(n: usize, seed: u32, count: usize) -> Vec<Vec<f32>> {
    let edges = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, -33.5, 533.0, -17066.0, 1e30, -1e30, 123.456,
    ];
    let mut v: Vec<Vec<f32>> = edges.iter().map(|&e| vec![e; n]).collect();
    let mut s = seed;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..count {
        let mut m = vec![0f32; n];
        for x in m.iter_mut() {
            let mut f = f32::from_bits(next());
            while f.is_nan() {
                f = f32::from_bits(next());
            }
            *x = f;
        }
        v.push(m);
    }
    v
}

/// The output and two inputs the vector tests share.
struct MinMaxEnv<'a> {
    emu: Emu<'a>,
    dst: u64,
    a: u64,
    b: u64,
}

fn minmax_env<'a>(wow: &Wow) -> MinMaxEnv<'a> {
    let mut emu = Emu::new(wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let dst = emu.alloc(0x10);
    let a = emu.alloc(0x10);
    let b = emu.alloc(0x10);
    MinMaxEnv { emu, dst, a, b }
}

fn put_vec(emu: &mut Emu, base: u64, v: &[f32]) {
    for (i, &f) in v.iter().enumerate() {
        emu.write_u32(base + i as u64 * 4, f.to_bits());
    }
}

#[test]
fn vec3_cross_world() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let MinMaxEnv { mut emu, dst, a, b } = minmax_env(&wow);
    let vs = synth_f32(3, 0xc505_5a11, 600);
    let mut checked = 0u32;
    for w in vs.windows(2) {
        let (av, bv): (&[f32; 3], &[f32; 3]) = (
            w[0].as_slice().try_into().unwrap(),
            w[1].as_slice().try_into().unwrap(),
        );
        put_vec(&mut emu, a, av);
        put_vec(&mut emu, b, bv);
        emu.call(0x672130, dst as u32, a as u32, &[b as u32]);
        let ours = benilla_re_terrain::vec3_cross_world(av, bv);
        for (i, &o) in ours.iter().enumerate() {
            assert_f32(emu.read_f32(dst + i as u64 * 4), o, "cross");
        }
        checked += 1;
    }
    eprintln!("vec3_cross_world: {checked} cases bit-exact vs WoW.exe @ 0x672130");
}

#[test]
fn vec3_normalize_world() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let MinMaxEnv { mut emu, dst, .. } = minmax_env(&wow);
    let vs = synth_f32(3, 0xc505_5a11, 600);
    let mut checked = 0u32;
    for w in vs.windows(2) {
        let av: &[f32; 3] = w[0].as_slice().try_into().unwrap();
        // normalize in place on a copy
        put_vec(&mut emu, dst, av);
        emu.call(0x6720f0, dst as u32, 0, &[]);
        let mut ours_n = *av;
        benilla_re_terrain::vec3_normalize_world(&mut ours_n);
        for (i, &o) in ours_n.iter().enumerate() {
            assert_f32(emu.read_f32(dst + i as u64 * 4), o, "normalize");
        }
        checked += 1;
    }
    eprintln!("vec3_normalize_world: {checked} cases bit-exact vs WoW.exe @ 0x6720f0");
}

#[test]
fn chunk_get_height() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.write_u32(0xc9e388, 0); // the x87 path
    let chunk = emu.alloc(0x1000);
    let hdr = emu.alloc(0x80);
    let point = emu.alloc(0x10);
    let out = emu.alloc(8);
    emu.write_u32(chunk + 0xf08, hdr as u32);
    let mut s: u32 = 0x9e11_a7e5;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let mut checked = 0u32;
    for case in 0..150u32 {
        // a spread of vertices, the heights random
        let mut verts = [[0f32; 3]; 145];
        for (k, v) in verts.iter_mut().enumerate() {
            let row = k / 17 * 2 + usize::from(k % 17 >= 9); // a spread, not the real layout
            let col = k % 17;
            v[0] = row as f32 * -2.083 + (next() % 100) as f32 * 1e-3;
            v[1] = col as f32 * -2.083 + (next() % 100) as f32 * 1e-3;
            v[2] = ((next() % 2000) as f32 - 1000.0) * 0.05;
        }
        let pos = [
            ((next() % 1000) as f32) - 500.0,
            ((next() % 1000) as f32) - 500.0,
            ((next() % 1000) as f32) * 0.1,
        ];
        let holes: u16 = if case % 5 == 0 {
            (next() & 0xffff) as u16
        } else {
            0
        };
        let cellx = next() & 7;
        let celly = next() & 7;
        let px = (f64::from(pos[0])
            + f64::from(verts[(celly as usize * 17 + cellx as usize) % 145][0])
            - 1.0) as f32;
        let py = (f64::from(pos[1])
            + f64::from(verts[(celly as usize * 17 + cellx as usize) % 145][1])
            - 1.0) as f32;
        for (k, v) in verts.iter().enumerate() {
            for (c, &vc) in v.iter().enumerate() {
                emu.write_u32(chunk + 0x83c + (k * 3 + c) as u64 * 4, vc.to_bits());
            }
        }
        emu.write_u32(chunk + 0x6c, pos[0].to_bits());
        emu.write_u32(chunk + 0x70, pos[1].to_bits());
        emu.write_u32(chunk + 0x74, pos[2].to_bits());
        emu.write_u32(hdr + 0x3c, u32::from(holes));
        emu.write_u32(point, px.to_bits());
        emu.write_u32(point + 4, py.to_bits());
        emu.write_u32(out, 0xdead_beef);
        let ret = emu.call(
            0x6b72a0,
            chunk as u32,
            point as u32,
            &[cellx, celly, out as u32],
        );
        let ours = benilla_re_terrain::chunk_get_height(&verts, holes, pos, cellx, celly, px, py);
        match ours {
            None => assert_eq!(ret, 0, "holed cell must return 0"),
            Some(h) => {
                assert_eq!(ret, 1, "clear cell must return 1");
                assert_f32(emu.read_f32(out), h, "chunk_get_height");
            }
        }
        checked += 1;
    }
    eprintln!("chunk_get_height: {checked} queries bit-exact vs WoW.exe @ 0x6b72a0 (x87 path)");
}

#[test]
fn liquid_height_sample() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let chunk = emu.alloc(0x1000);
    let slot = emu.alloc(0x40);
    let flags = emu.alloc(0x40);
    let mclq = emu.alloc(81 * 8);
    let point = emu.alloc(0x10);
    let out = emu.alloc(8);
    emu.write_u32(slot + 0xc, mclq as u32);
    emu.write_u32(slot + 0x10, flags as u32);
    emu.write_u32(slot + 0x14, 0); // flow count 0
    emu.write_u32(slot + 0x18, 0);
    let mut s: u32 = 0x11dd_55aa;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let mut checked = 0u32;
    for case in 0..200u32 {
        let present = case % 8 != 7;
        emu.write_u32(chunk + 0x118, if present { slot as u32 } else { 0 });
        emu.write_u32(chunk + 0x11c, 0);
        emu.write_u32(chunk + 0x120, 0);
        emu.write_u32(chunk + 0x124, 0);
        let mut cells = [0u8; 64];
        for c in cells.iter_mut() {
            *c = (next() >> 17) as u8;
        }
        let mut heights = [0f32; 81];
        for h in heights.iter_mut() {
            *h = ((next() % 4000) as f32 - 2000.0) * 0.07;
        }
        for (i, &c) in cells.iter().enumerate() {
            emu.write(flags + i as u64, &[c]);
        }
        for (i, &h) in heights.iter().enumerate() {
            emu.write_u32(mclq + i as u64 * 8 + 4, h.to_bits());
        }
        let x = ((next() % 850) as f32) * 0.01; // 0..8.5
        let y = ((next() % 850) as f32) * 0.01;
        let pz = 1e9f32; // above any height: the flow branch never runs
        emu.write_u32(point + 8, pz.to_bits());
        emu.write_u32(out, 0xdead_beef);
        let ret = emu.call(
            0x6b7500,
            chunk as u32,
            point as u32,
            &[x.to_bits(), y.to_bits(), out as u32],
        );
        let fa: &[u8; 64] = &cells;
        let ha: &[f32; 81] = &heights;
        let ours = benilla_re_terrain::liquid_height_sample(
            [
                if present { Some((fa, ha)) } else { None },
                None,
                None,
                None,
            ],
            x,
            y,
        );
        match ours {
            None => assert_eq!(ret, 0, "no liquid must return 0"),
            Some(h) => {
                assert_eq!(ret, 1, "liquid must return 1");
                let got = emu.read_f32(out);
                if got.to_bits() != h.to_bits() {
                    eprintln!(
                        "MISMATCH case={case} x={x:?} y={y:?} got={:#010x} ours={:#010x}",
                        got.to_bits(),
                        h.to_bits()
                    );
                    let rx = f64::from(x) - 0.5;
                    eprintln!("  ix(re)={} rx={rx:?}", rx.round_ties_even() as i32);
                }
                assert_f32(got, h, "liquid_height_sample");
            }
        }
        checked += 1;
    }
    eprintln!("liquid_height_sample: {checked} samples bit-exact vs WoW.exe @ 0x6b7500");
}

#[test]
fn liquid_status_world() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(0x69b520, 0, 0); // the map object query finds none
    let area = emu.alloc(0x700);
    let chunk = emu.alloc(0x1000);
    let slot = emu.alloc(0x40);
    let flags = emu.alloc(0x40);
    let mclq = emu.alloc(81 * 8);
    let pos = emu.alloc(0x10);
    let (t_out, h_out, f_out, g_out) = (emu.alloc(8), emu.alloc(8), emu.alloc(0x10), emu.alloc(8));
    emu.write_u32(slot + 0xc, mclq as u32);
    emu.write_u32(slot + 0x10, flags as u32);
    emu.write_u32(slot + 0x14, 0);
    emu.write_u32(slot + 0x18, 0);
    let mut s: u32 = 0x33aa_cc55;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let mut checked = 0u32;
    for case in 0..250u32 {
        // a position near tile (32, 32), so the indices stay in range
        let px = ((next() % 2000) as f32) * 0.1 - 100.0; // ±100 about the centre
        let py = ((next() % 2000) as f32) * 0.1 - 100.0;
        let pz = ((next() % 400) as f32) * 0.5 - 100.0;
        // the table indices the function computes, to place the test's world
        let wxs = (-(f64::from(py) - f64::from(benilla_re_terrain::MAP_CENTER))
            * f64::from(benilla_re_terrain::CELL_SCALE)) as f32;
        let wys = (-(f64::from(px) - f64::from(benilla_re_terrain::MAP_CENTER))
            * f64::from(benilla_re_terrain::CELL_SCALE)) as f32;
        let u3 = (f64::from(wxs) - 0.5).round_ties_even() as i32;
        let u4 = (f64::from(wys) - 0.5).round_ties_even() as i32;
        let area_idx = (((u4 >> 7) & 0x3f) * 0x40 + ((u3 >> 7) & 0x3f)) as u64;
        let chunk_idx = (((u4 >> 3) & 0xf) * 0x10 + ((u3 >> 3) & 0xf)) as u64;
        // clear the area table entries earlier cases set
        emu.write_u32(0xc96318 + area_idx * 4, area as u32);
        emu.write_u32(area + 0x278 + chunk_idx * 4, chunk as u32);
        let present = case % 6 != 5;
        emu.write_u32(chunk + 0x118, if present { slot as u32 } else { 0 });
        emu.write_u32(chunk + 0x11c, 0);
        emu.write_u32(chunk + 0x120, 0);
        emu.write_u32(chunk + 0x124, 0);
        let mut cells = [0u8; 64];
        for c in cells.iter_mut() {
            *c = (next() >> 13) as u8;
        }
        let mut heights = [0f32; 81];
        for h in heights.iter_mut() {
            *h = ((next() % 4000) as f32 - 2000.0) * 0.05;
        }
        for (i, &c) in cells.iter().enumerate() {
            emu.write(flags + i as u64, &[c]);
        }
        for (i, &h) in heights.iter().enumerate() {
            emu.write_u32(mclq + i as u64 * 8 + 4, h.to_bits());
        }
        emu.write_u32(pos, px.to_bits());
        emu.write_u32(pos + 4, py.to_bits());
        emu.write_u32(pos + 8, pz.to_bits());
        const SENT: u32 = 0xdead_beef;
        for &o in &[t_out, h_out, g_out] {
            emu.write_u32(o, SENT);
        }
        for i in 0..3 {
            emu.write_u32(f_out + i * 4, SENT);
        }
        let ret = emu.call(
            0x69b6d0,
            pos as u32,
            t_out as u32,
            &[h_out as u32, f_out as u32, g_out as u32],
        );
        let fa: &[u8; 64] = &cells;
        let ha: &[f32; 81] = &heights;
        let ours = benilla_re_terrain::liquid_status([px, py, pz], |ai, ci| {
            (ai as u64 == area_idx && ci as u64 == chunk_idx).then(|| {
                [
                    present.then_some(benilla_re_terrain::LiquidSlotData {
                        flags: fa,
                        heights: ha,
                        flow_count: 0,
                        zones: [benilla_re_terrain::SoundZone::default(); 2],
                    }),
                    None,
                    None,
                    None,
                ]
            })
        });
        assert_eq!(
            ret & 0xff,
            u32::from(ours.hit),
            "liquid_status ret (case {case})"
        );
        let expect_u32 = |o: Option<u32>| o.unwrap_or(SENT);
        assert_eq!(emu.read_u32(g_out), expect_u32(ours.fishable), "fishable");
        assert_eq!(emu.read_u32(t_out), expect_u32(ours.liquid_type), "type");
        assert_eq!(
            emu.read_u32(h_out),
            ours.height.map_or(SENT, f32::to_bits),
            "height (case {case})"
        );
        let f = ours.flow.unwrap_or([f32::from_bits(SENT); 3]);
        for (i, &fv) in f.iter().enumerate() {
            assert_eq!(
                emu.read_u32(f_out + i as u64 * 4),
                fv.to_bits(),
                "flow[{i}]"
            );
        }
        // clean the table entry for the next case (indices change per case)
        emu.write_u32(0xc96318 + area_idx * 4, 0);
        checked += 1;
    }
    eprintln!("liquid_status_world: {checked} queries bit-exact vs WoW.exe @ 0x69b6d0");
}

/// `0x69b6d0`'s flow for zone counts 1 and 2, with the zones at the slot's `+0x18` and the point
/// submerged: count 1 inside, on the radius (`dist² == radius²`, inside) and outside; count 2 in
/// both, in each alone and in neither.
#[test]
fn liquid_status_sound_zone() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(0x69b520, 0, 0); // the map object query finds none
    let area = emu.alloc(0x700);
    let chunk = emu.alloc(0x1000);
    let slot = emu.alloc(0x40);
    let flags = emu.alloc(0x40);
    let mclq = emu.alloc(81 * 8);
    let wave = emu.alloc(0x80);
    let pos = emu.alloc(0x10);
    let (t_out, h_out, f_out, g_out) = (emu.alloc(8), emu.alloc(8), emu.alloc(0x10), emu.alloc(8));
    emu.write_u32(slot + 0xc, mclq as u32);
    emu.write_u32(slot + 0x10, flags as u32);
    emu.write_u32(slot + 0x18, wave as u32);

    // a fixed position below the liquid height, so the flow is computed
    let (px, py, pz, height) = (50.0f32, -30.0f32, 5.0f32, 10.0f32);
    let wxs = (-(f64::from(py) - f64::from(benilla_re_terrain::MAP_CENTER))
        * f64::from(benilla_re_terrain::CELL_SCALE)) as f32;
    let wys = (-(f64::from(px) - f64::from(benilla_re_terrain::MAP_CENTER))
        * f64::from(benilla_re_terrain::CELL_SCALE)) as f32;
    let u3 = (f64::from(wxs) - 0.5).round_ties_even() as i32;
    let u4 = (f64::from(wys) - 0.5).round_ties_even() as i32;
    let area_idx = (((u4 >> 7) & 0x3f) * 0x40 + ((u3 >> 7) & 0x3f)) as u64;
    let chunk_idx = (((u4 >> 3) & 0xf) * 0x10 + ((u3 >> 3) & 0xf)) as u64;
    let (cx, cy) = ((u3 & 7) as usize, (u4 & 7) as usize);
    emu.write_u32(0xc96318 + area_idx * 4, area as u32);
    emu.write_u32(area + 0x278 + chunk_idx * 4, chunk as u32);
    emu.write_u32(chunk + 0x118, slot as u32);
    emu.write_u32(chunk + 0x11c, 0);
    emu.write_u32(chunk + 0x120, 0);
    emu.write_u32(chunk + 0x124, 0);
    // a type 0 cell, neither 0xf nor type 1, reaches the bilinear height and the flow
    let mut cells = [0u8; 64];
    cells[cy * 8 + cx] = 0;
    for (i, &c) in cells.iter().enumerate() {
        emu.write(flags + i as u64, &[c]);
    }
    let heights = [height; 81]; // a constant height, so the point is below it
    for (i, &h) in heights.iter().enumerate() {
        emu.write_u32(mclq + i as u64 * 8 + 4, h.to_bits());
    }
    emu.write_u32(pos, px.to_bits());
    emu.write_u32(pos + 4, py.to_bits());
    emu.write_u32(pos + 8, pz.to_bits());

    // zone 0 at +0x0 of the zone object, zone 1 at +0x28
    let plant_zone = |emu: &mut Emu, base: u64, z: &benilla_re_terrain::SoundZone| {
        for k in 0..3u64 {
            emu.write_u32(wave + base + k * 4, z.center[k as usize].to_bits());
        }
        emu.write_u32(wave + base + 0xc, z.radius.to_bits());
        for k in 0..3u64 {
            emu.write_u32(wave + base + 0x10 + k * 4, z.dir[k as usize].to_bits());
        }
        emu.write_u32(wave + base + 0x1c, z.scale.to_bits());
    };
    let mk = |off: [f32; 3], r: f32, dir: [f32; 3], scale: f32| benilla_re_terrain::SoundZone {
        center: [px + off[0], py + off[1], pz + off[2]],
        radius: r,
        dir,
        scale,
    };
    let (d0, d1, s0, s1) = ([1.0f32, 2.0, 3.0], [-1.0f32, 0.5, 2.0], 0.5f32, 1.5f32);
    let z = benilla_re_terrain::SoundZone::default;
    // (count, zone 0, zone 1, label): an offset of 0.5 is inside (radius 2), 2.0 on the radius
    // (radius 2) and 5.0 outside (radius 1).
    let cases: [(
        u32,
        benilla_re_terrain::SoundZone,
        benilla_re_terrain::SoundZone,
        &str,
    ); 7] = [
        (1, mk([0.5, 0.0, 0.0], 2.0, d0, s0), z(), "count1 inside"),
        (
            1,
            mk([2.0, 0.0, 0.0], 2.0, d0, s0),
            z(),
            "count1 ==boundary",
        ),
        (1, mk([5.0, 0.0, 0.0], 1.0, d0, s0), z(), "count1 outside"),
        (
            2,
            mk([0.5, 0.0, 0.0], 2.0, d0, s0),
            mk([0.0, 0.5, 0.0], 2.0, d1, s1),
            "count2 both",
        ),
        (
            2,
            mk([0.5, 0.0, 0.0], 2.0, d0, s0),
            mk([5.0, 0.0, 0.0], 1.0, d1, s1),
            "count2 z0-only",
        ),
        (
            2,
            mk([5.0, 0.0, 0.0], 1.0, d0, s0),
            mk([0.0, 0.5, 0.0], 2.0, d1, s1),
            "count2 z1-only",
        ),
        (
            2,
            mk([5.0, 0.0, 0.0], 1.0, d0, s0),
            mk([6.0, 0.0, 0.0], 1.0, d1, s1),
            "count2 neither",
        ),
    ];
    const SENT: u32 = 0xdead_beef;
    let mut checked = 0u32;
    for (count, z0, z1, label) in cases {
        emu.write_u32(slot + 0x14, count);
        plant_zone(&mut emu, 0x0, &z0);
        plant_zone(&mut emu, 0x28, &z1);
        for &o in &[t_out, h_out, g_out] {
            emu.write_u32(o, SENT);
        }
        for i in 0..3 {
            emu.write_u32(f_out + i * 4, SENT);
        }
        let ret = emu.call(
            0x69b6d0,
            pos as u32,
            t_out as u32,
            &[h_out as u32, f_out as u32, g_out as u32],
        );
        let fa: &[u8; 64] = &cells;
        let ha: &[f32; 81] = &heights;
        let zones = [z0, z1];
        let ours = benilla_re_terrain::liquid_status([px, py, pz], |ai, ci| {
            (ai as u64 == area_idx && ci as u64 == chunk_idx).then_some([
                Some(benilla_re_terrain::LiquidSlotData {
                    flags: fa,
                    heights: ha,
                    flow_count: count,
                    zones,
                }),
                None,
                None,
                None,
            ])
        });
        assert_eq!(ret & 0xff, u32::from(ours.hit), "{label}: hit");
        let f = ours.flow.unwrap_or([f32::from_bits(SENT); 3]);
        for (i, &fv) in f.iter().enumerate() {
            assert_eq!(
                emu.read_u32(f_out + i as u64 * 4),
                fv.to_bits(),
                "{label}: flow[{i}]"
            );
        }
        checked += 1;
    }
    eprintln!(
        "liquid_status_sound_zone: {checked} flow cases (count 1/2, 4-way + ==boundary) bit-exact vs WoW.exe @ 0x69b6d0"
    );
}

#[test]
fn ground_effect_lookup() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let area = emu.alloc(0x700);
    let chunk = emu.alloc(0x1000);
    let hdr = emu.alloc(0x80);
    let lowres = emu.alloc(0x10);
    let layer_bufs: Vec<u64> = (0..4).map(|_| emu.alloc(0x10)).collect();
    let gtab = emu.alloc(0x100); // the ground effect row pointers
    let grow = emu.alloc(0x20); // one effect row
    let pos = emu.alloc(8);
    let out = emu.alloc(8);
    emu.write_u32(chunk + 0xf08, hdr as u32);
    emu.write_u32(chunk + 0xf0c, lowres as u32);
    let mut s: u32 = 0x9696_1234;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let mut checked = 0u32;
    for case in 0..250u32 {
        let px = ((next() % 2000) as f32) * 0.1 - 100.0;
        let py = ((next() % 2000) as f32) * 0.1 - 100.0;
        let wx = (-(f64::from(py) - f64::from(benilla_re_terrain::MAP_CENTER))) as f32;
        let u7 = (f64::from(benilla_re_terrain::CELL_SCALE) * f64::from(wx) - 0.5).round_ties_even()
            as i32;
        let wy = (-(f64::from(px) - f64::from(benilla_re_terrain::MAP_CENTER))) as f32;
        let u5 = (f64::from(benilla_re_terrain::CELL_SCALE) * f64::from(wy) - 0.5).round_ties_even()
            as i32;
        let area_idx = (((u5 >> 7) & 0x3f) * 0x40 + ((u7 >> 7) & 0x3f)) as u64;
        let chunk_idx = (((u5 >> 3) & 0xf) * 0x10 + ((u7 >> 3) & 0xf)) as u64;
        emu.write_u32(0xc96318 + area_idx * 4, area as u32);
        emu.write_u32(area + 0x278 + chunk_idx * 4, chunk as u32);
        let holes: u16 = if case % 4 == 0 {
            (next() & 0xffff) as u16
        } else {
            0
        };
        emu.write_u32(hdr + 0x3c, u32::from(holes));
        let layer_count = next() % 5;
        emu.write_u32(chunk + 0x138, layer_count);
        let mut lr = [0u16; 8];
        for v in lr.iter_mut() {
            *v = (next() & 0xffff) as u16;
        }
        for (i, &v) in lr.iter().enumerate() {
            emu.write_u32(lowres + i as u64 * 2, u32::from(v));
        }
        let mut effects = [0u16; 4];
        for (i, e) in effects.iter_mut().enumerate() {
            *e = match next() % 4 {
                0 => 0xffff,
                1 => 200, // beyond the table count
                _ => (next() % 4) as u16,
            };
            emu.write_u32(chunk + 0x128 + i as u64 * 4, layer_bufs[i] as u32);
            emu.write_u32(layer_bufs[i] + 0xa, u32::from(*e) | 0x5a5a_0000);
        }
        let table_count: i32 = 4;
        emu.write_u32(0xc0dc64, table_count as u32);
        emu.write_u32(0xc0dc60, gtab as u32);
        let row_value = 0x600d_0000 | case;
        for k in 0..5u64 {
            let present = k % 2 == 0;
            emu.write_u32(gtab + k * 4, if present { grow as u32 } else { 0 });
        }
        emu.write_u32(grow + 0x18, row_value);
        emu.write_u32(pos, px.to_bits());
        emu.write_u32(pos + 4, py.to_bits());
        emu.write_u32(out, 0xdead_beef);
        let ret = emu.call(0x69b1c0, pos as u32, out as u32, &[]);
        let lra: &[u16; 8] = &lr;
        let ours = benilla_re_terrain::ground_effect_lookup(
            [px, py],
            table_count,
            |ai, ci| {
                (ai as u64 == area_idx && ci as u64 == chunk_idx).then_some(
                    benilla_re_terrain::GroundChunk {
                        holes,
                        layer_count,
                        lowres: lra,
                        layer_effects: effects,
                    },
                )
            },
            |e| (u64::from(e) % 2 == 0).then_some(row_value),
        );
        match ours {
            None => assert_eq!(ret & 0xff, 0, "ground_effect miss (case {case})"),
            Some(v) => {
                assert_eq!(ret & 0xff, 1, "ground_effect hit (case {case})");
                assert_eq!(emu.read_u32(out), v, "ground_effect row value");
            }
        }
        emu.write_u32(0xc96318 + area_idx * 4, 0);
        checked += 1;
    }
    eprintln!("ground_effect_lookup: {checked} lookups bit-exact vs WoW.exe @ 0x69b1c0");
}

/// `0x6b72a0` on a constructed cell: the corners on one plane and the inner vertex lifted, so each
/// of the four triangles gives a different height, with a point inside each, and points on the
/// diagonals, where a test is exactly zero and counts as true (`0x6b7326`, `0x6b7358`); the two
/// triangles there share the edge, so the height agrees either way in exact arithmetic.
#[test]
fn chunk_get_height_quadrants() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.write_u32(0xc9e388, 0); // the x87 path
    let chunk = emu.alloc(0x1000);
    let hdr = emu.alloc(0x80);
    let point = emu.alloc(0x10);
    let out = emu.alloc(8);
    emu.write_u32(chunk + 0xf08, hdr as u32);

    // Cell (0, 0): vertices 0, 1, 9, 17 and 18, a unit square with its corners on z = 2u + 3v + 10
    // and the inner vertex at 20.
    let zc = |u: f32, v: f32| -> f32 { 2.0 * u + 3.0 * v + 10.0 };
    let mut verts = [[0f32; 3]; 145];
    verts[0] = [0.0, 0.0, zc(0.0, 0.0)]; // the top-left corner
    verts[1] = [0.0, 1.0, zc(0.0, 1.0)]; // +1
    verts[9] = [0.5, 0.5, 20.0]; // the inner vertex, above the corners' 12.5
    verts[17] = [1.0, 0.0, zc(1.0, 0.0)]; // +17
    verts[18] = [1.0, 1.0, zc(1.0, 1.0)]; // +18
    let pos = [0.0f32, 0.0, 7.0]; // so the point is its own offset
    for (k, v) in verts.iter().enumerate() {
        for (c, &vc) in v.iter().enumerate() {
            emu.write_u32(chunk + 0x83c + (k * 3 + c) as u64 * 4, vc.to_bits());
        }
    }
    emu.write_u32(chunk + 0x6c, pos[0].to_bits());
    emu.write_u32(chunk + 0x70, pos[1].to_bits());
    emu.write_u32(chunk + 0x74, pos[2].to_bits());
    emu.write_u32(hdr + 0x3c, 0); // no holes

    // Test one is px − py and test two 1 − px − py here, each true at <= 0: q is 1 for the first
    // and 2 for the second.
    let queries: [(f32, f32, u32); 7] = [
        (0.5, 0.25, 0),
        (0.125, 0.5, 1),
        (0.875, 0.6, 2),
        (0.5, 0.875, 3),
        (0.3, 0.3, 1),   // test one exactly zero
        (0.75, 0.25, 2), // test two exactly zero
        (0.5, 0.5, 3),   // both exactly zero
    ];
    let cellx = 0u32;
    let celly = 0u32;
    let mut checked = 0u32;
    for (px, py, q) in queries {
        emu.write_u32(point, px.to_bits());
        emu.write_u32(point + 4, py.to_bits());
        emu.write_u32(out, 0xdead_beef);
        let ret = emu.call(
            0x6b72a0,
            chunk as u32,
            point as u32,
            &[cellx, celly, out as u32],
        );
        let ours = benilla_re_terrain::chunk_get_height(&verts, 0, pos, cellx, celly, px, py);
        let h = ours.expect("clear cell must return a height");
        assert_eq!(ret, 1, "clear cell must return 1 (q={q})");
        assert_f32(
            emu.read_f32(out),
            h,
            &format!("chunk_get_height_quadrants q={q}"),
        );
        checked += 1;
    }
    eprintln!(
        "chunk_get_height_quadrants: {checked} triangles and diagonals bit-exact vs WoW.exe @ 0x6b72a0"
    );
}
