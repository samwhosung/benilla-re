//! The small functions, each run whole: `DistanceSquared` (`0x69be70`), `SetPlaneFromNormal`
//! (`0x6acde0`), `LerpColor` (`0x690ae0`), `FadeGroupColors` (`0x6b4090`) with `0x6d48b0` stubbed
//! to return the fade object, `LerpFogRecord` (`0x69efd0`), `ScaleMatrix3` (`0x672d80`) and
//! `GetAreaId` (`0x69b120`) on an area table the test lays out.

#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::leaves::{
    self, area_id_lookup, color_fade_modulate_i, color_lerp_i, fade_to_i, lerp_color_rec,
    mat3_scale_world, plane_from_np, vec3_dist2, AreaWorld, ColorRec,
};
use common::assert_f32;

/// Random finite floats after a set of edge values; NaN is left out, as `fld`/`fstp` would quiet a
/// signalling one.
fn synth_f32(n: usize, seed: u32, count: usize) -> Vec<Vec<f32>> {
    let edges = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, -33.5, 533.0, -17066.0, 17066.666, 1e30, -1e30, 123.456,
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

fn put(emu: &mut Emu, base: u64, v: &[f32]) {
    for (i, &f) in v.iter().enumerate() {
        emu.write_u32(base + i as u64 * 4, f.to_bits());
    }
}

/// The area table in the emulator's memory (tiles at `0xc96318`, a tile's chunks at `+0x278`, a
/// chunk's area id at `+0x16c`), read through [`AreaWorld`].
struct EmuWorld<'a, 'b> {
    emu: &'a Emu<'b>,
}

impl AreaWorld for EmuWorld<'_, '_> {
    fn tile_ptr(&self, grid_index: usize) -> u32 {
        self.emu
            .read_u32(u64::from(leaves::AREA_TABLE_VA) + grid_index as u64 * 4)
    }
    fn chunk_ptr(&self, tile: u32, chunk_index: usize) -> u32 {
        self.emu
            .read_u32(u64::from(tile) + 0x278 + chunk_index as u64 * 4)
    }
    fn area_id(&self, chunk: u32) -> u32 {
        self.emu.read_u32(u64::from(chunk) + 0x16c)
    }
}

/// `0x69be70` has no branches. Its result is left in `st0`, so the call's store rounds it, and the
/// reimplementation's `f64` is rounded the same way. The inputs include zero and huge differences.
#[test]
fn vec3_dist2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let a = emu.alloc(0x10);
    let b = emu.alloc(0x10);
    let vs = synth_f32(3, 0x1d15_7a2c, 700);
    let mut checked = 0u32;
    for w in vs.windows(2) {
        let (av, bv): (&[f32; 3], &[f32; 3]) = (
            w[0].as_slice().try_into().unwrap(),
            w[1].as_slice().try_into().unwrap(),
        );
        put(&mut emu, a, av);
        put(&mut emu, b, bv);
        // ecx = a, b on the stack, `ret 4`; the result in st0
        let binary = emu.call_f32(0x69be70, a as u32, 0, &[b as u32]);
        let ours = vec3_dist2(av, bv) as f32;
        assert_f32(binary, ours, "vec3_dist2");
        checked += 1;
    }
    eprintln!("vec3_dist2: {checked} cases bit-exact vs WoW.exe @ 0x69be70");
}

/// `0x6acde0` has no branches: the normal is copied bit for bit, signed zeros and NaN included.
#[test]
fn plane_from_np_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let out = emu.alloc(0x20);
    let n = emu.alloc(0x10);
    let p = emu.alloc(0x10);
    let ns = synth_f32(3, 0x9211_ce04, 700);
    let mut checked = 0u32;
    for w in ns.windows(2) {
        let (nv, pv): (&[f32; 3], &[f32; 3]) = (
            w[0].as_slice().try_into().unwrap(),
            w[1].as_slice().try_into().unwrap(),
        );
        put(&mut emu, n, nv);
        put(&mut emu, p, pv);
        emu.write_u32(out + 0xc, 0xdead_beef);
        // ecx = out, n and p on the stack, `ret 8`
        emu.call(0x6acde0, out as u32, 0, &[n as u32, p as u32]);
        let ours = plane_from_np(nv, pv);
        for i in 0..4 {
            assert_f32(
                emu.read_f32(out + i as u64 * 4),
                ours[i],
                &format!("plane[{i}]"),
            );
        }
        checked += 1;
    }
    eprintln!("plane_from_np: {checked} cases bit-exact vs WoW.exe @ 0x6acde0");
}

/// `0x690ae0` at `t` 0xff and at every other `t`, with each byte's difference of either sign: `t` of
/// 0, 1, 0x7f, 0x80, 0xfe and 0xff, and random ones.
#[test]
fn color_lerp_i_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let dst = emu.alloc(8);
    let src = emu.alloc(8);
    let mut s: u32 = 0xc010_2030;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let ts: Vec<u32> = {
        let mut v = vec![0u32, 1, 0x40, 0x7f, 0x80, 0xc0, 0xfe, 0xff];
        for _ in 0..16 {
            v.push(next() & 0xff);
        }
        v
    };
    let mut checked = 0u32;
    for _ in 0..200u32 {
        let dv = next();
        let sv = next();
        for &t in &ts {
            emu.write_u32(dst, dv);
            emu.write_u32(src, sv);
            // ecx = dst, in and out; t and src on the stack, `ret 8`
            emu.call(0x690ae0, dst as u32, 0, &[t, src as u32]);
            let binary = emu.read_u32(dst);
            let ours = color_lerp_i(dv, t, sv);
            assert_eq!(
                binary, ours,
                "color_lerp_i dst={dv:#010x} src={sv:#010x} t={t}"
            );
            checked += 1;
        }
    }
    eprintln!("color_lerp_i: {checked} cases bit-exact vs WoW.exe @ 0x690ae0");
}

/// `0x6b4090` with no elements, and with elements flagged and not; `0x6d48b0` stubbed to return the
/// fade object, whose fade at `+0x1ac` runs from below zero, where the factor is negative, to about
/// one.
#[test]
fn color_fade_modulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let this = emu.alloc(0x200);
    let fade_obj = emu.alloc(0x200);
    let elems = emu.alloc(0x40 * 8); // 8 elements, stride 0x40
    emu.stub(0x6d48b0, fade_obj as u32, 0); // returns the fade object
    emu.write_u32(this + 0x1d8, elems as u32);
    let mut s: u32 = 0xfade_1234;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let fades = [
        0.0f32, 0.5, 1.0, 0.25, 0.999, 1.5, -0.3, 0.0019607, 0.9999, 2.0,
    ];
    let counts = [0u32, 1, 4, 8];
    let mut checked = 0u32;
    for &count in &counts {
        for &fade in &fades {
            emu.write_u32(this + 0x1dc, count);
            emu.write_u32(fade_obj + 0x1ac, fade.to_bits());
            // half the elements flagged with 0x10
            let mut rgbs = [0u32; 8];
            let mut prev10 = [0u32; 8];
            let mut flagged = [false; 8];
            for e in 0..8usize {
                let base = elems + (e as u64) * 0x40;
                let flag = if e % 2 == 0 { 0x10u32 } else { 0x00 };
                flagged[e] = flag != 0;
                emu.write_u32(base, flag);
                let v10 = next();
                prev10[e] = v10;
                emu.write_u32(base + 0x10, v10);
                let rgb = next();
                rgbs[e] = rgb;
                emu.write_u32(base + 0x14, rgb);
            }
            // ecx = this
            emu.call(0x6b4090, this as u32, 0, &[]);
            let i = fade_to_i(fade);
            for e in 0..(count as usize) {
                let base = elems + (e as u64) * 0x40;
                let got = emu.read_u32(base + 0x14);
                if flagged[e] {
                    // `+0x10` is copied to `+0x14` first, so the colour faded is `+0x10`'s.
                    let ours = color_fade_modulate_i(prev10[e], i);
                    assert_eq!(
                        got, ours,
                        "fade_modulate elem={e} fade={fade} i={i} src10={:#010x}",
                        prev10[e]
                    );
                } else {
                    assert_eq!(got, rgbs[e], "fade_modulate unflagged elem={e} untouched");
                }
            }
            checked += 1;
        }
    }
    eprintln!("color_fade_modulate: {checked} runs bit-exact vs WoW.exe @ 0x6b4090");
}

/// `0x69efd0` with the colour factor at 0 (`t <= 0`), at 0xff (`t >= 1`) and between, and random
/// `t` and colours.
#[test]
fn lerp_color_rec_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let dst = emu.alloc(0x10); // {f32 x, f32 y, u32 rgba}
    let src = emu.alloc(0x10);
    let mut s: u32 = 0x1ec0_10aa;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let ts = [
        -1.0f32, -0.001, 0.0, 0.001, 0.25, 0.5, 0.75, 0.999, 1.0, 1.5, 2.0,
    ];
    let coords = synth_f32(2, 0x44cc_88aa, 60);
    let mut checked = 0u32;
    for (ci, cw) in coords.windows(2).enumerate() {
        let dxy = &cw[0];
        let sxy = &cw[1];
        let drgba = next();
        let srgba = next();
        for &t in &ts {
            emu.write_u32(dst, dxy[0].to_bits());
            emu.write_u32(dst + 4, dxy[1].to_bits());
            emu.write_u32(dst + 8, drgba);
            emu.write_u32(src, sxy[0].to_bits());
            emu.write_u32(src + 4, sxy[1].to_bits());
            emu.write_u32(src + 8, srgba);
            // ecx = dst, in and out; src and t on the stack, `ret 8`
            emu.call(0x69efd0, dst as u32, 0, &[src as u32, t.to_bits()]);
            let ours = lerp_color_rec(
                ColorRec {
                    x: dxy[0],
                    y: dxy[1],
                    rgba: drgba,
                },
                ColorRec {
                    x: sxy[0],
                    y: sxy[1],
                    rgba: srgba,
                },
                t,
            );
            assert_f32(emu.read_f32(dst), ours.x, "lerp_color_rec.x");
            assert_f32(emu.read_f32(dst + 4), ours.y, "lerp_color_rec.y");
            assert_eq!(
                emu.read_u32(dst + 8),
                ours.rgba,
                "lerp_color_rec.rgba ci={ci} t={t} d={drgba:#010x} s={srgba:#010x}"
            );
            checked += 1;
        }
    }
    eprintln!("lerp_color_rec: {checked} cases bit-exact vs WoW.exe @ 0x69efd0");
}

/// `0x672d80` has no branches; the scale includes 0, ±1, huge values and subnormals.
#[test]
fn mat3_scale_world_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let out = emu.alloc(0x40);
    let src = emu.alloc(0x40);
    let mats = synth_f32(9, 0x33aa_77cc, 400);
    let scalars = synth_f32(1, 0x88bb_22dd, 0); // edge scalars only
    let mut checked = 0u32;
    for m in &mats {
        let mv: &[f32; 9] = m.as_slice().try_into().unwrap();
        for sw in &scalars {
            let sc = sw[0];
            put(&mut emu, src, mv);
            for i in 0..9 {
                emu.write_u32(out + i as u64 * 4, 0xdead_beef);
            }
            // ecx = out, edx = src, the scale on the stack, `ret 4`
            emu.call(0x672d80, out as u32, src as u32, &[sc.to_bits()]);
            let ours = mat3_scale_world(mv, sc);
            for i in 0..9 {
                assert_f32(
                    emu.read_f32(out + i as u64 * 4),
                    ours[i],
                    &format!("mat3_scale[{i}] sc={sc}"),
                );
            }
            checked += 1;
        }
    }
    eprintln!("mat3_scale_world: {checked} cases bit-exact vs WoW.exe @ 0x672d80");
}

#[test]
fn area_id_lookup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    // clear the 64×64 tile pointers at 0xc96318
    for i in 0..0x1000u64 {
        emu.write_u32(leaves::AREA_TABLE_VA as u64 + i * 4, 0);
    }
    // a few tiles and chunks, a sparse fixed subset of them set
    let tile_a = emu.alloc(0x2000); // 0x278 + 256*4 chunk table
    let tile_b = emu.alloc(0x2000);
    let chunk_a = emu.alloc(0x200);
    let chunk_c = emu.alloc(0x200);
    emu.write_u32(chunk_a + 0x16c, 0x1111_2222);
    emu.write_u32(chunk_c + 0x16c, 0x3333_4444);
    let pos = emu.alloc(0x10);

    let mut s: u32 = 0xa7ea_1d00;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    // the centre, near the corners, and random positions over the map
    let mut positions: Vec<(f32, f32)> = vec![
        (17066.666, 17066.666),
        (0.0, 0.0),
        (34133.0, 34133.0),
        (533.33, 533.33),
        (1000.0, 25000.0),
        (16000.0, 18000.0),
    ];
    for _ in 0..60 {
        let x = (next() % 34133) as f32 + (next() % 1000) as f32 * 0.001;
        let y = (next() % 34133) as f32 + (next() % 1000) as f32 * 0.001;
        positions.push((x, y));
    }

    // Random tiles and chunks are set, so some positions find no tile, some a tile without the
    // chunk, and some the chunk.
    for gi in 0..0x1000u64 {
        // a third of the cells hold a tile
        if next() % 3 == 0 {
            let t = if next() & 1 == 0 { tile_a } else { tile_b };
            emu.write_u32(leaves::AREA_TABLE_VA as u64 + gi * 4, t as u32);
        }
    }
    // Half the chunk slots of both tiles are set, chosen by a high bit of the LCG: its low bit
    // alternates, and a choice by `% 2` that draws again when true would set every slot.
    for t in [tile_a, tile_b] {
        for ci in 0..256u64 {
            let v = if (next() >> 8) % 2 == 0 {
                if (next() >> 8) & 1 == 0 {
                    chunk_a as u32
                } else {
                    chunk_c as u32
                }
            } else {
                0
            };
            emu.write_u32(t + 0x278 + ci * 4, v);
        }
    }

    // The reads are counted, so each path is shown to run: no tile, a tile without the chunk, and
    // the chunk. A result of 0 alone does not tell the first two apart.
    use std::cell::Cell;
    struct CountWorld<'a, 'b> {
        inner: EmuWorld<'a, 'b>,
        tile_zero: Cell<u32>,
        chunk_zero: Cell<u32>,
        resolved: Cell<u32>,
    }
    impl AreaWorld for CountWorld<'_, '_> {
        fn tile_ptr(&self, grid_index: usize) -> u32 {
            let t = self.inner.tile_ptr(grid_index);
            if t == 0 {
                self.tile_zero.set(self.tile_zero.get() + 1);
            }
            t
        }
        fn chunk_ptr(&self, tile: u32, chunk_index: usize) -> u32 {
            let c = self.inner.chunk_ptr(tile, chunk_index);
            if c == 0 {
                self.chunk_zero.set(self.chunk_zero.get() + 1);
            } else {
                self.resolved.set(self.resolved.get() + 1);
            }
            c
        }
        fn area_id(&self, chunk: u32) -> u32 {
            self.inner.area_id(chunk)
        }
    }

    let mut checked = 0u32;
    let (mut path_a, mut path_b, mut path_c) = (0u32, 0u32, 0u32);
    for (x, y) in positions {
        emu.write_u32(pos, x.to_bits());
        emu.write_u32(pos + 4, y.to_bits());
        // ecx = pos
        let binary = emu.call(0x69b120, pos as u32, 0, &[]);
        let world = CountWorld {
            inner: EmuWorld { emu: &emu },
            tile_zero: Cell::new(0),
            chunk_zero: Cell::new(0),
            resolved: Cell::new(0),
        };
        let ours = area_id_lookup(&world, x, y);
        path_a += world.tile_zero.get();
        path_b += world.chunk_zero.get();
        path_c += world.resolved.get();
        assert_eq!(binary, ours, "area_id_lookup x={x} y={y}");
        checked += 1;
    }
    assert!(path_a > 0, "expected path A (no tile) to fire");
    assert!(path_b > 0, "expected path B (tile, no chunk) to fire");
    assert!(path_c > 0, "expected path C (resolved area id) to fire");
    eprintln!(
        "area_id_lookup: {checked} positions bit-exact vs WoW.exe @ 0x69b120 \
         (paths A={path_a} B={path_b} C={path_c})"
    );
}
