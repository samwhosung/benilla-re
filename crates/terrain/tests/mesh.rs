//! The vertex builders, each run whole: `MinVector3` (`0x699250`), `MaxVector3` (`0x6992c0`),
//! `MaxVector3InPlace` (`0x6b11f0`), `CMapChunk::BuildVertices` (`0x6b0e50`) on a chunk the test
//! lays out, and `CChunkLiquid::BuildVertices` (`0x68d540`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::assert_f32;

fn assert_slice(binary: &[f32], ours: &[f32], ctx: &str) {
    assert_eq!(binary.len(), ours.len());
    for i in 0..binary.len() {
        assert_f32(binary[i], ours[i], &format!("{ctx}[{i}]"));
    }
}

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

/// Chunk indices: edge values and random ones in 0..=1023 per axis, 64 tiles of 16 chunks.
fn synth_indices() -> Vec<(i32, i32)> {
    let mut v = vec![
        (0, 0),
        (1, 0),
        (0, 1),
        (15, 15),
        (512, 512),
        (1023, 1023),
        (33, 700),
    ];
    let mut s: u32 = 0xdead_beef;
    for _ in 0..25 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        v.push(((s >> 8) as i32 & 0x3ff, (s >> 20) as i32 & 0x3ff));
    }
    v
}

/// The output and two inputs the vector min and max tests share.
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

fn get3(emu: &Emu, base: u64) -> [f32; 3] {
    [
        emu.read_f32(base),
        emu.read_f32(base + 4),
        emu.read_f32(base + 8),
    ]
}

#[test]
fn vec3_min() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let MinMaxEnv { mut emu, dst, a, b } = minmax_env(&wow);
    let vs = synth_f32(3, 0x1357_9bdf, 800);
    let mut checked = 0u32;
    for w in vs.windows(2) {
        let (av, bv): (&[f32; 3], &[f32; 3]) = (
            w[0].as_slice().try_into().unwrap(),
            w[1].as_slice().try_into().unwrap(),
        );
        put_vec(&mut emu, a, av);
        put_vec(&mut emu, b, bv);
        emu.call(0x699250, dst as u32, a as u32, &[b as u32]);
        assert_slice(
            &get3(&emu, dst),
            &benilla_re_terrain::vec3_min(av, bv),
            "vec3_min",
        );
        checked += 1;
    }
    eprintln!("vec3_min: {checked} cases bit-exact vs WoW.exe @ 0x699250");
}

#[test]
fn vec3_max() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let MinMaxEnv { mut emu, dst, a, b } = minmax_env(&wow);
    let vs = synth_f32(3, 0x1357_9bdf, 800);
    let mut checked = 0u32;
    for w in vs.windows(2) {
        let (av, bv): (&[f32; 3], &[f32; 3]) = (
            w[0].as_slice().try_into().unwrap(),
            w[1].as_slice().try_into().unwrap(),
        );
        put_vec(&mut emu, a, av);
        put_vec(&mut emu, b, bv);
        emu.call(0x6992c0, dst as u32, a as u32, &[b as u32]);
        assert_slice(
            &get3(&emu, dst),
            &benilla_re_terrain::vec3_max(av, bv),
            "vec3_max",
        );
        checked += 1;
    }
    eprintln!("vec3_max: {checked} cases bit-exact vs WoW.exe @ 0x6992c0");
}

#[test]
fn vec3_max_inplace() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let MinMaxEnv {
        mut emu, dst, b, ..
    } = minmax_env(&wow);
    let vs = synth_f32(3, 0x1357_9bdf, 800);
    let mut checked = 0u32;
    for w in vs.windows(2) {
        let (av, bv): (&[f32; 3], &[f32; 3]) = (
            w[0].as_slice().try_into().unwrap(),
            w[1].as_slice().try_into().unwrap(),
        );
        put_vec(&mut emu, b, bv);
        // ecx is the in-out vector, the stack argument the other
        put_vec(&mut emu, dst, av);
        emu.call(0x6b11f0, dst as u32, 0, &[b as u32]);
        let mut ours = *av;
        benilla_re_terrain::vec3_max_inplace(&mut ours, bv);
        assert_slice(&get3(&emu, dst), &ours, "vec3_max_inplace");
        checked += 1;
    }
    eprintln!("vec3_max_inplace: {checked} cases bit-exact vs WoW.exe @ 0x6b11f0");
}

#[test]
fn mcvt_world_verts() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let chunk = emu.alloc(0x1000); // CMapChunk is 0xf34
    let mcvt = emu.alloc(145 * 4);
    let heights_sets = synth_f32(145, 0x2468_ace0, 60);
    let positions = synth_f32(3, 0x0f1e_2d3c, 60);
    let indices = synth_indices();
    let mut checked = 0u32;
    for (n, hs) in heights_sets.iter().enumerate() {
        let (ix, iy) = indices[n % indices.len()];
        let pos: [f32; 3] = positions[n % positions.len()]
            .as_slice()
            .try_into()
            .unwrap();
        for i in 0..0x1000 / 4 {
            emu.write_u32(chunk + i * 4, 0);
        }
        emu.write_u32(chunk + 0x8c, ix as u32);
        emu.write_u32(chunk + 0x90, iy as u32);
        emu.write_u32(chunk + 0x6c, pos[0].to_bits());
        emu.write_u32(chunk + 0x70, pos[1].to_bits());
        emu.write_u32(chunk + 0x74, pos[2].to_bits());
        emu.write_u32(chunk + 0xf14, mcvt as u32);
        for (i, &h) in hs.iter().enumerate() {
            emu.write_u32(mcvt + i as u64 * 4, h.to_bits());
        }
        emu.call(0x6b0e50, chunk as u32, 0, &[]);
        let ours =
            benilla_re_terrain::mcvt_world_verts(ix, iy, pos, hs.as_slice().try_into().unwrap());
        let read_n = |base: u64, n: usize| -> Vec<f32> {
            (0..n).map(|i| emu.read_f32(base + i as u64 * 4)).collect()
        };
        assert_slice(&read_n(chunk + 0x44, 3), &ours.aabb_min, "mcvt aabb_min");
        assert_slice(&read_n(chunk + 0x50, 3), &ours.aabb_max, "mcvt aabb_max");
        assert_slice(&read_n(chunk + 0x5c, 3), &ours.center, "mcvt center");
        assert_f32(emu.read_f32(chunk + 0x68), ours.radius, "mcvt radius");
        assert_slice(
            &read_n(chunk + 0x94, 6),
            &ours.bounds_copy,
            "mcvt bounds_copy",
        );
        let flat: Vec<f32> = ours.verts.iter().flatten().copied().collect();
        assert_slice(&read_n(chunk + 0x83c, 145 * 3), &flat, "mcvt verts");
        checked += 1;
    }
    eprintln!("mcvt_world_verts: {checked} chunks bit-exact vs WoW.exe @ 0x6b0e50");
}

#[test]
fn liquid_grid_verts() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let liq = emu.alloc(0x400); // liquid object: +0xc mclq ptr, +0x1c chunk ptr, +0x34 out verts
    let chunk = emu.alloc(0x100); // only +0x8c/+0x90 (ix/iy) and +0x74 (height_base) are read
    let mclq = emu.alloc(81 * 8); // 81 × {u32 at +0, f32 height at +4}
    let heights_sets = synth_f32(81, 0x5a5a_5a5a, 60);
    let bases = synth_f32(1, 0x7777_1111, 60);
    let indices = synth_indices();
    let mut checked = 0u32;
    for (n, hs) in heights_sets.iter().enumerate() {
        let (ix, iy) = indices[n % indices.len()];
        let base = bases[n % bases.len()][0];
        emu.write_u32(liq + 0xc, mclq as u32);
        emu.write_u32(liq + 0x1c, chunk as u32);
        emu.write_u32(chunk + 0x8c, ix as u32);
        emu.write_u32(chunk + 0x90, iy as u32);
        emu.write_u32(chunk + 0x74, base.to_bits());
        for (i, &h) in hs.iter().enumerate() {
            emu.write_u32(mclq + i as u64 * 8, 0xaaaa_5555); // the unread +0 dword
            emu.write_u32(mclq + i as u64 * 8 + 4, h.to_bits());
        }
        emu.call(0x68d540, liq as u32, 0, &[]);
        let ours =
            benilla_re_terrain::liquid_grid_verts(ix, iy, base, hs.as_slice().try_into().unwrap());
        let flat: Vec<f32> = ours.iter().flatten().copied().collect();
        let got: Vec<f32> = (0..81 * 3)
            .map(|i| emu.read_f32(liq + 0x34 + i as u64 * 4))
            .collect();
        assert_slice(&got, &flat, "liquid verts");
        checked += 1;
    }
    eprintln!("liquid_grid_verts: {checked} grids bit-exact vs WoW.exe @ 0x68d540");
}
