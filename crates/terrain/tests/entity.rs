//! The arithmetic of the map entities, as slices: `CWorld::IsPointInEntity` (`0x670a70`),
//! `LinkEntityToMapObject` (`0x6a8ed0`), `LinkEntityToChunks` (`0x6a8ca0`),
//! `CWorld::UpdateFadingModels` (`0x672ef0`), `AttachEntityToGround` (`0x6a8a20`),
//! `BakeMapEntityGroupLight` (`0x69e4c0`), `CMapDoodadDef::BakeGroupLight` (`0x6a8410`),
//! `CreateWorldCallbackObject` (`0x670db0`), `UpdateMapEntityEnvironment` (`0x69e280`) and
//! `PlaceDoodadDef` (`0x694bc0`). A stretch run in an island is the exe's own bytes, checked,
//! copied between a load and a store.

#![allow(clippy::type_complexity)] // the cases are tuples of the fields they write

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::entity::{
    clamp01_to_color, cmapdoodaddef_ground_down, cmapdoodaddef_ground_up, cmapobj_skyocc_offsets,
    cmapobj_vert_offsets, def_chunk_window, defvol_classify, defvol_neg_offset,
    doodad_despawn_fade, floordrop_down_probe, floordrop_lerp, floordrop_offset, mddf_create_xform,
    MddfRecord, MddfXform,
};
use common::{assert_f32, frand, fuzz_f32, fuzz_wide, next, SLICE_ESP};

/// A random `f32` in `[-1000, 1000)`.
fn fuzz_near_probe(s: &mut u32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (m * 2.0e3 - 1.0e3) as f32 // [-1000, 1000)
}

/// The map's half extent, 17066.666.
const GRID_MAX: f32 = f32::from_bits(0x4685_5555);

/// The chunk window's probes: the half extent, zeros, distinct corners, and coordinates whose
/// `0.03·r − 0.5` is a half integer.
fn chunk_window_probes(e: &mut Emu, run_case: &dyn Fn(&mut Emu, f32, f32, f32, f32, &str)) {
    // at the grid max: 17066.666 − coord == 0 → product 0 → fistp(−0.5) → 0 (RNE of −0.5 is 0).
    run_case(
        e,
        GRID_MAX,
        GRID_MAX,
        GRID_MAX,
        GRID_MAX,
        "grid-max (tile_rel 0)",
    );
    run_case(e, 0.0, 0.0, 0.0, 0.0, "all-zero");
    run_case(e, 16000.0, 15000.0, 16000.0, 15000.0, "mid-tile");
    run_case(e, 17066.0, 0.0, 8533.0, -100.0, "spread");
    // distinct corners so the LIVE-st0 (max_x) path and the f32-reload (others) path diverge if
    // wrongly modelled: max_x kept LIVE-f64, the rest rounded f32 by their fchs;fstp.
    run_case(e, 30000.0, 1000.0, 25000.0, 2000.0, "distinct corners");

    // round-to-even boundary sweep: pick coords so prod = tile_rel·0.03 lands prod−0.5 on .5.
    for k in 0..64i32 {
        let tile_rel = (k as f32) / 0.03 + 0.5 / 0.03; // prod ≈ k + 0.5 → prod − 0.5 ≈ k
        let coord = GRID_MAX - tile_rel;
        run_case(e, coord, coord, coord, coord, &format!("rne[{k}]"));
    }
}

/// A random box over the map, minimum at most maximum.
fn fuzz_bounds(s: &mut u32) -> (f32, f32, f32, f32) {
    let a = frand(s, -2000.0, 34000.0);
    let b = frand(s, -2000.0, 34000.0);
    let c = frand(s, -2000.0, 34000.0);
    let d = frand(s, -2000.0, 34000.0);
    (a.max(b), a.min(b), c.max(d), c.min(d))
}

/// A finite `f32` over five ranges, from 1e-6 to 1e6.
fn fuzz_fade(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 5;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 20.0 - 10.0) as f32,
        2 => (m * 2000.0 - 1000.0) as f32,
        3 => (m * 1.0e6) as f32,
        _ => (m * 1.0e-6) as f32,
    }
}

/// A finite `f32` over six ranges, the last near ±1000.
fn fuzz_offsets(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 20.0 - 10.0) as f32,
        2 => (m * 2000.0 - 1000.0) as f32,
        3 => (m * 1.0e6 - 5.0e5) as f32,
        4 => (m * 1.0e-6) as f32,
        _ => (m * 2.0e3 - 1.0e3) as f32, // near the ±1000/±1760 const magnitudes (cancellation)
    }
}

/// A colour block of `0x670db0` or `0x69e280` from `begin` to `until`: the components at `eax`, the
/// word read from the object's `+0xf4` (`esi`).
fn color_block_difftest(e: &mut Emu, begin: u64, until: u64, tag: &str) {
    let src = e.alloc(0x40); // EAX: colour source [c0, c1, c2]
    let obj = e.alloc(0x400); // ESI: object (writes [esi+0x24..],[esi+0xf4], etc.)

    let run_case = |e: &mut Emu, c: [f32; 3], ctx: &str| {
        for (k, &f) in c.iter().enumerate() {
            e.write_u32(src + 4 * k as u64, f.to_bits());
        }
        e.write_u32(obj + 0xf4, 0xdead_beef); // junk-seed: a non-write would diverge
        e.set_reg(RegisterX86::EAX, src);
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(begin, until);
        let bin = e.read_u32(obj + 0xf4);
        let ours = clamp01_to_color(c);
        assert_eq!(
            bin, ours,
            "{tag} {ctx}: binary={bin:#010x} ours={ours:#010x}"
        );
    };

    // exact / clamp-boundary probes: each component crosses 0.0, 1.0, and the interior `·255+0.5`
    // truncation grid (the `__ftol` round-toward-zero boundary lands at the .0 of each byte step).
    run_case(e, [0.0, 0.0, 0.0], "all-zero → 0xff000000 (clamp-0)");
    run_case(e, [1.0, 1.0, 1.0], "all-one → 0xffffffff (clamp-255)");
    run_case(e, [0.5, 0.5, 0.5], "all-half → 0.5·255+0.5=128");
    run_case(
        e,
        [f32::NAN, f32::INFINITY, f32::NEG_INFINITY],
        "nan, infinities",
    );
    run_case(e, [-1.0, 2.0, 0.5], "mixed clamp lo/hi/interior");
    run_case(
        e,
        [-0.0, 1.0, 1.5],
        "neg-zero clamp-0, ==1 clamp-255, >1 clamp-255",
    );
    // ==threshold component probes (just-below-1, just-above-0, exact byte-step interiors).
    run_case(
        e,
        [
            f32::from_bits(0x3f7f_ffff),
            f32::from_bits(0x0000_0001),
            0.999,
        ],
        "just<1 / tiny>0 / near-255",
    );
    for k in 0..=255u32 {
        // c so c·255+0.5 lands on integer k → truncates to k; sweep the whole byte range.
        let c = (k as f32) / 255.0;
        run_case(e, [c, c, c], &format!("byte-sweep[{k}]"));
    }

    let mut s = 0x646f_6f64u32 ^ (begin as u32); // "dood" mixed with the row VA
    for i in 0..6000u32 {
        // mix of in-band [0,1], straddle [-0.2,1.2], and wide fuzz (exercises both clamps +
        // NaN/inf).
        let comp = |s: &mut u32| match (next(s) >> 8) % 4 {
            0 => frand(s, 0.0, 1.0),
            1 => frand(s, -0.2, 1.2),
            2 => fuzz_f32(s),
            _ => frand(s, -2.0, 3.0),
        };
        let c = [comp(&mut s), comp(&mut s), comp(&mut s)];
        run_case(e, c, &format!("{tag} fuzz[{i}] c={c:?}"));
    }
}

/// `0x670a70` at `0x670b3c`, `fchs` between a load and a store.
#[test]
fn defvol_neg_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // The `fchs` at all five negate sites are the bytes the copy runs.
    for addr in [
        0x0067_0b3cu64,
        0x0067_0b8a,
        0x0067_0bd5,
        0x0067_0c20,
        0x0067_0c6b,
    ] {
        assert_eq!(
            e.read(addr, 2),
            vec![0xd9, 0xe0],
            "defvol fchs bytes changed @ {addr:#x}"
        );
    }
    let fchs = e.read(0x0067_0b3c, 2); // fchs (the representative, byte-verified == the family)

    let in_slot = e.alloc(0x10);
    let out_slot = e.alloc(0x10);
    let island = e.alloc(0x40);

    // island: fld dword [in_slot]; <fchs>; fstp dword [out_slot]
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&[0xd9, 0x05]); // fld dword [abs32]
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(&fchs); // d9 e0
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [abs32]
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    e.write(island, &code);
    let island_end = island + code.len() as u64;

    let run_case = |e: &mut Emu, x: f32, ctx: &str| {
        e.write_u32(in_slot, x.to_bits());
        e.write_u32(out_slot, 0xdead_beef); // junk-seed so a non-write would diverge
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(island, island_end);
        assert_f32(e.read_f32(out_slot), defvol_neg_offset(x), ctx);
    };

    // exact / boundary probes: 0 (−0.0), ±1, large, tiny.
    run_case(&mut e, 0.0, "x=0 → -0.0");
    run_case(&mut e, 1.0, "x=1 → -1");
    run_case(&mut e, -1.0, "x=-1 → 1");
    run_case(&mut e, 1234.5, "x=1234.5 → -1234.5");
    run_case(&mut e, 1.0e30, "huge");
    run_case(&mut e, 1.0e-30, "tiny");
    let mut s = 0x0607_0b3cu32;
    for i in 0..5000u32 {
        let x = fuzz_wide(&mut s);
        run_case(&mut e, x, &format!("neg fuzz[{i}] x={x}"));
    }
}

/// `0x670a70` from `0x670ca9` to `0x670cbd` in an island with a store appended: the plane at
/// `ecx-4`, the point at `ebx`.
#[test]
fn defvol_classify_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // The contiguous FP run [0x670ca6, 0x670cbd): fld [ecx-4]; fmul [ebx]; fld [ecx+4];
    // fmul [ebx+8]; faddp; fld [ecx]; fmul [ebx+4]; faddp; fadd [ecx+8].
    let dot = e.read(0x0067_0ca6, 0x17); // 23 bytes, up to (not incl.) the fcomp @670cbd
    assert_eq!(
        dot,
        vec![
            0xd9, 0x41, 0xfc, // fld dword [ecx-0x4]
            0xd8, 0x0b, // fmul dword [ebx]
            0xd9, 0x41, 0x04, // fld dword [ecx+0x4]
            0xd8, 0x4b, 0x08, // fmul dword [ebx+0x8]
            0xde, 0xc1, // faddp st(1), st
            0xd9, 0x01, // fld dword [ecx]
            0xd8, 0x4b, 0x04, // fmul dword [ebx+0x4]
            0xde, 0xc1, // faddp st(1), st
            0xd8, 0x41, 0x08, // fadd dword [ecx+0x8]
        ],
        "defvol_classify dot bytes changed"
    );

    let plane = e.alloc(0x20); // a/b/c/d at [ecx-4]/[ecx]/[ecx+4]/[ecx+8]
    let point = e.alloc(0x20); // px/py/pz at [ebx]/[ebx+4]/[ebx+8]
    let out_slot = e.alloc(0x10);
    let island = e.alloc(0x40);

    // ecx must point so [ecx-4]=a is in-bounds: put a 4 bytes into `plane`, so ecx = plane+4.
    let ecx = plane + 4;

    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&dot);
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [abs32]
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    e.write(island, &code);
    let island_end = island + code.len() as u64;

    let run_case = |e: &mut Emu, pl: [f32; 4], pt: [f32; 3], ctx: &str| {
        // plane layout: [ecx-4]=a, [ecx]=b, [ecx+4]=c, [ecx+8]=d  → ecx=plane+4.
        e.write_u32(plane, pl[0].to_bits()); // a = [ecx-4]
        e.write_u32(plane + 4, pl[1].to_bits()); // b = [ecx]
        e.write_u32(plane + 8, pl[2].to_bits()); // c = [ecx+4]
        e.write_u32(plane + 0xc, pl[3].to_bits()); // d = [ecx+8]
        e.write_u32(point, pt[0].to_bits()); // px = [ebx]
        e.write_u32(point + 4, pt[1].to_bits()); // py = [ebx+4]
        e.write_u32(point + 8, pt[2].to_bits()); // pz = [ebx+8]
        e.write_u32(out_slot, 0xdead_beef);
        e.set_reg(RegisterX86::ECX, ecx);
        e.set_reg(RegisterX86::EBX, point);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(island, island_end);
        assert_f32(e.read_f32(out_slot), defvol_classify(pl, pt), ctx);
    };

    // probes: on-plane (dist==0), front, back, zero normal, diag near-zero.
    run_case(
        &mut e,
        [1.0, 0.0, 0.0, -5.0],
        [5.0, 7.0, 9.0],
        "on-plane (dist==0)",
    );
    run_case(&mut e, [1.0, 0.0, 0.0, 0.0], [5.0, 7.0, 9.0], "front");
    run_case(&mut e, [1.0, 0.0, 0.0, 0.0], [-5.0, 7.0, 9.0], "back");
    run_case(&mut e, [0.0, 0.0, 0.0, 0.0], [5.0, 7.0, 9.0], "zero-normal");
    run_case(
        &mut e,
        [0.577, 0.577, 0.577, -34.62],
        [10.0, 20.0, 30.0],
        "diag near-zero",
    );
    let mut s = 0x0607_0ca6u32;
    for i in 0..6000u32 {
        let pl = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        let pt = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        run_case(&mut e, pl, pt, &format!("classify fuzz[{i}]"));
    }
}

/// `0x6a8ed0` at `0x6a8f00`: the height at `[ebp-0x2c]` less 1000.
#[test]
fn floordrop_down_probe_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame: z and the result at [ebp-0x2c]
    let ebp = frame + 0x80;
    let obj = e.alloc(0x80); // EAX: `mov ecx,[eax+4]`@6a8f0c (an int copy — value irrelevant, must map)

    let run_case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 0x2c, z.to_bits());
        e.set_reg(RegisterX86::EAX, obj); // so `mov ecx,[eax+4]`@6a8f0c doesn't fault
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a8efd, 0x6a8f12);
        assert_f32(e.read_f32(ebp - 0x2c), floordrop_down_probe(z), ctx);
    };

    // exact / ==threshold probes around the const magnitude.
    run_case(&mut e, 0.0, "z=0 → -1000.0");
    run_case(&mut e, 1000.0, "z=1000 → 0.0 exact");
    run_case(&mut e, -1000.0, "z=-1000 → -2000.0");
    run_case(&mut e, f32::from_bits(0x447a_0001), "z just >1000");
    run_case(&mut e, 1.0e20, "huge");
    let mut s = 0x6a8e_d0d0u32;
    for i in 0..5000u32 {
        let z = if i % 3 == 0 {
            fuzz_near_probe(&mut s)
        } else {
            fuzz_wide(&mut s)
        };
        run_case(&mut e, z, &format!("down fuzz[{i}] z={z}"));
    }
}

/// `0x6a8ed0` at `0x6a9093`: the height at `[ebp-0x38]` plus 1000, read from `[ebp-0x58]`.
#[test]
fn floordrop_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame: z at [ebp-0x38]; result at [ebp-0x58]
    let ebp = frame + 0x80;
    let obj = e.alloc(0x100); // ESI: `mov edx,[esi+0x7c]`@6a9099 (an int copy — value irrelevant)

    let run_case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 0x38, z.to_bits()); // z = [ebp-0x38]
        e.write_u32(ebp - 0x40, 0x4011_1111); // [ebp-0x40] copy operand (any value)
        e.write_u32(ebp - 0x3c, 0x4022_2222); // [ebp-0x3c] copy operand
        e.write_u32(ebp - 0x58, 0xdead_beef); // junk-seed the result slot
        e.set_reg(RegisterX86::ESI, obj); // so `mov edx,[esi+0x7c]`@6a9099 doesn't fault
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a908d, 0x6a90bd);
        assert_f32(e.read_f32(ebp - 0x58), floordrop_offset(z), ctx);
    };

    // exact / ==threshold probes around the const magnitude.
    run_case(&mut e, 0.0, "z=0 → 1000.0");
    run_case(&mut e, -1000.0, "z=-1000 → 0.0 exact");
    run_case(&mut e, 1000.0, "z=1000 → 2000.0");
    run_case(&mut e, f32::from_bits(0xc47a_0001), "z just <-1000");
    run_case(&mut e, 1.0e20, "huge");
    let mut s = 0x6a90_8d8du32;
    for i in 0..5000u32 {
        let z = if i % 3 == 0 {
            fuzz_near_probe(&mut s)
        } else {
            fuzz_wide(&mut s)
        };
        run_case(&mut e, z, &format!("offset fuzz[{i}] z={z}"));
    }
}

/// `0x6a8ed0` from `0x6a90c9` to `0x6a9128`: the start at `[ebp-0x24]`, the target at `[ebp-0x34]`,
/// `t` at `[ebp-0x8]`; the point is read from `[ebp-0x54]`.
#[test]
fn floordrop_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame: base/to/t inputs and out results
    let ebp = frame + 0x80;
    let outvec = e.alloc(0x20); // [ebp+0x10] target of the `mov [edx],eax` write-through

    let run_case = |e: &mut Emu, base: [f32; 3], to: [f32; 3], t: f32, ctx: &str| {
        e.write_u32(ebp - 0x24, base[0].to_bits()); // Sx
        e.write_u32(ebp - 0x20, base[1].to_bits()); // Sy
        e.write_u32(ebp - 0x1c, base[2].to_bits()); // Sz
        e.write_u32(ebp - 0x34, to[0].to_bits()); // Tx
        e.write_u32(ebp - 0x30, to[1].to_bits()); // Ty
        e.write_u32(ebp - 0x2c, to[2].to_bits()); // Tz
        e.write_u32(ebp - 0x8, t.to_bits()); // t
        e.write_u32(ebp + 0x10, outvec as u32); // edx target (write-through)
                                                // junk-seed the result slots so a non-write would
                                                // diverge.
        e.write_u32(ebp - 0x54, 0xdead_beef);
        e.write_u32(ebp - 0x50, 0xdead_beef);
        e.write_u32(ebp - 0x4c, 0xdead_beef);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a90c9, 0x6a9128);
        let bin = [
            e.read_f32(ebp - 0x54), // out_x
            e.read_f32(ebp - 0x50), // out_y
            e.read_f32(ebp - 0x4c), // out_z
        ];
        let ours = floordrop_lerp(base, to, t);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // exact / ==threshold + LIVE-vs-reload probes.
    // t=0 → out==base; t=1 → out==to; t=0.5 midpoint.
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        [10.0, 20.0, 30.0],
        0.0,
        "t=0 → base",
    );
    run_case(&mut e, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], 1.0, "t=1 → to");
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [10.0, 20.0, 30.0],
        0.5,
        "t=0.5 midpoint",
    );
    // the real call has base==to in x,y (down-probe only moves z); the down-probe target z=Sz-1000.
    run_case(
        &mut e,
        [100.0, 200.0, 50.0],
        [100.0, 200.0, -950.0],
        0.25,
        "real-shape (x,y fixed; z down 1000)",
    );
    // a diff with a long binary expansion so the X (f32-reloaded diff) and Y/Z (f64-live diff)
    // paths diverge in the low bits.
    run_case(
        &mut e,
        [
            f32::from_bits(0x3fab_cdef),
            f32::from_bits(0x3fab_cdef),
            f32::from_bits(0x3fab_cdef),
        ],
        [
            f32::from_bits(0x4abc_def1),
            f32::from_bits(0x4abc_def1),
            f32::from_bits(0x4abc_def1),
        ],
        f32::from_bits(0x3f2a_aaab), // ~0.666…
        "live-vs-reload low-bit straddle (X vs Y/Z)",
    );
    let mut s = 0x6a90_c9c9u32;
    for i in 0..6000u32 {
        let base = [fuzz_wide(&mut s), fuzz_wide(&mut s), fuzz_wide(&mut s)];
        let to = [fuzz_wide(&mut s), fuzz_wide(&mut s), fuzz_wide(&mut s)];
        // t mostly in [0,1] (the real hit fraction), some out-of-range for arithmetic coverage.
        let t = if i % 4 == 0 {
            fuzz_wide(&mut s)
        } else {
            frand(&mut s, 0.0, 1.0)
        };
        run_case(&mut e, base, to, t, &format!("lerp fuzz[{i}]"));
    }
}

/// `0x6a8ca0` from `0x6a8cbf` to `0x6a8d56`: the box at `ecx+0x44`; the four indices are read from
/// `[ebp-0xc]`, `[ebp-0x14]`, `[ebp-0x20]` and `[ebp-0x10]`. A NaN corner stores the integer
/// indefinite.
#[test]
fn def_chunk_window_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let aabb = e.alloc(0x80); // ecx = &def (bounds at +0x44..+0x54)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, max_x: f32, min_x: f32, max_y: f32, min_y: f32, ctx: &str| {
        e.write_u32(aabb + 0x44, min_x.to_bits());
        e.write_u32(aabb + 0x48, max_x.to_bits());
        e.write_u32(aabb + 0x50, min_y.to_bits());
        e.write_u32(aabb + 0x54, max_y.to_bits());
        e.set_reg(RegisterX86::ECX, aabb);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a8cbf, 0x6a8d56); // corner flips + window map + 4× fistp; halt at `mov ebx,[ebp-0x10]`
        let bin = [
            e.read_u32(ebp - 0xc) as i32,  // i_max_x
            e.read_u32(ebp - 0x14) as i32, // i_min_x
            e.read_u32(ebp - 0x20) as i32, // i_max_y
            e.read_u32(ebp - 0x10) as i32, // i_min_y
        ];
        let ours = def_chunk_window(max_x, min_x, max_y, min_y);
        assert_eq!(bin, ours, "{ctx}: binary={bin:?} ours={ours:?}");
    };

    chunk_window_probes(&mut e, &run_case);
    run_case(
        &mut e,
        f32::NAN,
        0.0,
        f32::NAN,
        f32::INFINITY,
        "nan and infinity",
    );
    let mut s = 0x6a8c_a0a0u32;
    for i in 0..3000u32 {
        let (max_x, min_x, max_y, min_y) = fuzz_bounds(&mut s);
        run_case(
            &mut e,
            max_x,
            min_x,
            max_y,
            min_y,
            &format!("def fuzz[{i}]"),
        );
    }
}

/// `0x672ef0` from `0x672fd3` to `0x67302b`: the age at `[ebp-4]`, the weight at `ebx+8`; the alpha
/// is read from `[ebp-8]`. Ages and weights put `t` at 0 and 1 exactly and a step either side, and
/// at NaN.
#[test]
fn doodad_despawn_fade_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x200); // EBX: weight @ +0x8
    let frame = e.alloc(0x100); // EBP frame: age @ ebp-0x4, result @ ebp-0x8
    let ebp = frame + 0x80;

    // run the whole leg; the internal jp/jne clamp branches stay in-range. Halt at
    // 0x67302b (`mov edx,[ebp-8]`), PAST all three result stores. age = [ebp-4] (int), weight =
    // [ebx+8] (f32), result = [ebp-8] (f32).
    let run_case = |e: &mut Emu, age: i32, weight: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, age as u32);
        e.write_u32(buf + 0x8, weight.to_bits());
        e.set_reg(RegisterX86::EBX, buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x672fd3, 0x67302b);
        let bin = e.read_f32(ebp - 0x8);
        let ours = doodad_despawn_fade(age, weight);
        assert_f32(bin, ours, ctx);
    };

    // RATE = 0.0005. t = (1 - age*RATE)*weight. The age band is the gated [0, 2000) window
    // (`iVar5 - puVar7[1]` is `cmp eax,0x7d0 / jle`, so age in [0, 0x7d0]).
    //
    // ==threshold / clamp-boundary probes:
    run_case(
        &mut e,
        0,
        1.0,
        "age=0 → t=1.0 exact (smoothstep at upper edge → 1)",
    ); // t==1
    run_case(
        &mut e,
        2000,
        1.0,
        "age=2000 → t=0.0 exact (smoothstep at lower edge → 0)",
    ); // t==0
    run_case(&mut e, 1000, 1.0, "age=1000 → t=0.5 interior (→ 0.5)"); // t=0.5
    run_case(&mut e, 0, 2.0, "weight=2 → t=2.0 (clamp to 1)"); // t>1
    run_case(&mut e, 0, -1.0, "weight=-1 → t=-1.0 (clamp to 0)"); // t<0
    run_case(&mut e, 2000, 0.0, "weight=0 → t=0 (smoothstep 0 → 0)"); // t=0
    run_case(&mut e, 500, 0.0, "weight=0, age=500 → t=0"); // t=0
    run_case(&mut e, 2001, 1.0, "age=2001 → t slightly <0 (clamp to 0)"); // t just below 0
    run_case(&mut e, -1, 1.0, "age=-1 → t slightly >1 (clamp to 1)"); // t just above 1
    run_case(&mut e, 1500, 1.0, "age=1500 → t=0.25"); // interior
    run_case(&mut e, 500, 1.0, "age=500 → t=0.75"); // interior
                                                    // The lower compare reads the register and the
                                                    // kept value the stored f32: a tiny weight
                                                    // keeps t near the f32 grain.
    run_case(&mut e, 1234, 1.0e-7, "tiny weight near +0 (boundary)");
    run_case(
        &mut e,
        1234,
        -1.0e-7,
        "tiny weight near -0 (lower-clamp boundary)",
    );
    run_case(
        &mut e,
        0,
        f32::from_bits(0x3f80_0001),
        "weight just >1 (t just >1)",
    );
    run_case(
        &mut e,
        0,
        f32::from_bits(0x3f7f_ffff),
        "weight just <1 (t just <1, smoothstep)",
    );
    // NaN weight → NaN through smoothstep.
    run_case(&mut e, 1000, f32::NAN, "weight NaN → NaN smoothstep");

    let mut s = 0x6465_6670u32; // "defp"
    for i in 0..6000u32 {
        // age across (and a little beyond) the gated window so both clamp legs fire; mix in a wider
        // band of weights so t spans <0, [0,1], >1, NaN-free.
        let age = match (next(&mut s) >> 8) % 4 {
            0 => (next(&mut s) % 2002) as i32,        // in-window [0,2001]
            1 => (next(&mut s) % 4001) as i32 - 1000, // straddles both clamps
            2 => (next(&mut s) % 0x7d1) as i32,       // exact gated window [0,0x7d0]
            _ => (next(&mut s) as i32) % 8000 - 4000, // wide
        };
        let weight = match (next(&mut s) >> 8) % 4 {
            0 => frand(&mut s, -0.5, 2.0), // straddles the [0,1] band after t-scale
            1 => frand(&mut s, 0.0, 1.0),
            2 => fuzz_fade(&mut s),
            _ => frand(&mut s, -2.0, 2.0),
        };
        run_case(
            &mut e,
            age,
            weight,
            &format!("fade fuzz[{i}] age={age} w={weight}"),
        );
    }
}

/// `0x69e4c0` at `0x69e4f8` and `0x69e515`, each in an island between a load and a store.
#[test]
fn cmapobj_vert_offsets_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame: z = [ebp-0x14]
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, z: f32, ctx: &str| {
        // fadd island: fld [ebp-0x14]; fadd 1.0; (push ecx); fstp [esp]. result at SLICE_ESP-4.
        e.write_u32(ebp - 0x14, z.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x69e4f5, 0x69e508);
        let bin_add = e.read_f32(SLICE_ESP - 4);

        // fsub island: fld [ebp-0x14]; fsub 12.0; (push ecx); fstp [esp]. result at SLICE_ESP-4.
        e.write_u32(ebp - 0x14, z.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x69e512, 0x69e525);
        let bin_sub = e.read_f32(SLICE_ESP - 4);

        let (ours_add, ours_sub) = cmapobj_vert_offsets(z);
        assert_f32(bin_add, ours_add, &format!("{ctx} (+1.0)"));
        assert_f32(bin_sub, ours_sub, &format!("{ctx} (-12.0)"));
    };

    // exact / ==threshold probes around the const magnitudes.
    run_case(&mut e, 0.0, "z=0 → (1.0, -12.0)");
    run_case(&mut e, 12.0, "z=12 → (13.0, 0.0 exact)");
    run_case(&mut e, -1.0, "z=-1 → (0.0 exact, -13.0)");
    run_case(&mut e, 1.0, "z=1 → (2.0, -11.0)");
    run_case(&mut e, -12.0, "z=-12 → (-11.0, -24.0)");
    run_case(&mut e, f32::from_bits(0x3f80_0001), "z just >1");
    run_case(&mut e, 1.0e20, "huge");
    run_case(&mut e, -1.0e20, "huge neg");
    run_case(&mut e, 1.0e-20, "tiny");

    let mut s = 0x7665_7274u32; // "vert"
    for i in 0..6000u32 {
        let z = fuzz_offsets(&mut s);
        run_case(&mut e, z, &format!("vert fuzz[{i}] z={z}"));
    }
}

/// `0x6a8410` at `0x6a8436` and `0x6a8475`, each in an island between a load and a store.
#[test]
fn cmapobj_skyocc_offsets_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200); // EBX: z = [ebx+0x2c]
    let frame = e.alloc(0x100); // EBP frame: fsub result at [ebp-0x14]
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(obj + 0x2c, z.to_bits());
        // fadd island: fld [ebx+0x2c]; fadd 1.5; (push ecx); fstp [esp]. result at SLICE_ESP-4.
        e.set_reg(RegisterX86::EBX, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a8430, 0x6a8443);
        let bin_add = e.read_f32(SLICE_ESP - 4);

        // fsub island: fld [ebx+0x2c]; fsub 1760.0; fstp [ebp-0x14]. no push; result at [ebp-0x14].
        e.write_u32(ebp - 0x14, 0xdead_beef); // junk-seed so a non-write would diverge
        e.set_reg(RegisterX86::EBX, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a846f, 0x6a8481);
        let bin_sub = e.read_f32(ebp - 0x14);

        let (ours_add, ours_sub) = cmapobj_skyocc_offsets(z);
        assert_f32(bin_add, ours_add, &format!("{ctx} (+1.5)"));
        assert_f32(bin_sub, ours_sub, &format!("{ctx} (-1760.0)"));
    };

    // exact / ==threshold probes around the const magnitudes.
    run_case(&mut e, 0.0, "z=0 → (1.5, -1760.0)");
    run_case(&mut e, 1760.0, "z=1760 → (1761.5, 0.0 exact)");
    run_case(&mut e, -1.5, "z=-1.5 → (0.0 exact, -1761.5)");
    run_case(&mut e, 1.5, "z=1.5 → (3.0, -1758.5)");
    run_case(&mut e, -1760.0, "z=-1760 → (-1758.5, -3520.0)");
    run_case(&mut e, f32::from_bits(0x44dc_0001), "z just >1760");
    run_case(&mut e, 1.0e20, "huge");
    run_case(&mut e, -1.0e20, "huge neg");
    run_case(&mut e, 1.0e-20, "tiny");

    let mut s = 0x736b_7900u32; // "sky\0"
    for i in 0..6000u32 {
        let z = fuzz_offsets(&mut s);
        run_case(&mut e, z, &format!("skyocc fuzz[{i}] z={z}"));
    }
}

/// `0x6a8a20` at `0x6a8a54`, and from `0x6a8ab8` to `0x6a8add` in an island.
#[test]
fn cmapdoodaddef_ground_attach_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200); // ESI: this; +0xa8+8 (down z), +0x68 (src z), +0x5c+8 (probe z)
    let frame = e.alloc(0x100); // EBP frame: [ebp-0x20] holds the probe/down z and the results
    let ebp = frame + 0x80;

    // DOWN leg: z − 1000.0. The island starts at 0x6a8a51 (fld [ebp-0x20]); [ebp-0x20] is set from
    // [esi+0xa8+8] in the prologue (0x6a8a4a/0x6a8a4e) which is BEFORE the island — so we seed
    // [ebp-0x20] directly with z.
    let run_down = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 0x20, z.to_bits());
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a8a51, 0x6a8a69);
        let bin = e.read_f32(ebp - 0x20);
        assert_f32(bin, cmapdoodaddef_ground_down(z), ctx);
    };

    // UP leg: (src_z + 1000.0) + probe_z. The island starts at 0x6a8ab0 (fld [esi+0x68]); inside it
    // [ebp-0x20] is set from eax = [esi+0x5c+8] (the probe z) at 0x6a8ace. So seed [esi+0x68]=src_z
    // and [esi+0x5c+8]=probe_z.
    let run_up = |e: &mut Emu, src_z: f32, probe_z: f32, ctx: &str| {
        e.write_u32(obj + 0x68, src_z.to_bits());
        e.write_u32(obj + 0x5c + 0x8, probe_z.to_bits());
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a8ab0, 0x6a8ae0);
        let bin = e.read_f32(ebp - 0x20);
        assert_f32(bin, cmapdoodaddef_ground_up(src_z, probe_z), ctx);
    };

    // DOWN exact / ==threshold probes.
    run_down(&mut e, 0.0, "down z=0 → -1000.0");
    run_down(&mut e, 1000.0, "down z=1000 → 0.0 exact");
    run_down(&mut e, -1000.0, "down z=-1000 → -2000.0");
    run_down(&mut e, f32::from_bits(0x447a_0001), "down z just >1000");
    run_down(&mut e, 1.0e20, "down huge");

    // UP exact / ==threshold + LIVE-chain probes.
    run_up(&mut e, 0.0, 0.0, "up (0,0) → 1000.0");
    run_up(&mut e, -1000.0, 0.0, "up (-1000,0) → 0.0 exact");
    run_up(&mut e, 0.0, -1000.0, "up (0,-1000) → 0.0 exact");
    run_up(&mut e, -1000.0, -1000.0, "up (-1000,-1000) → -1000.0");
    run_up(
        &mut e,
        1.0e7,
        1.0e-3,
        "up large+tiny (LIVE-f64 chain straddle)",
    );
    run_up(&mut e, -1.0e7, 1.0e7, "up cancellation");

    let mut s = 0x6772_6e64u32; // "grnd"
    for i in 0..6000u32 {
        let z = fuzz_offsets(&mut s);
        run_down(&mut e, z, &format!("down fuzz[{i}] z={z}"));
        let src_z = fuzz_offsets(&mut s);
        let probe_z = fuzz_offsets(&mut s);
        run_up(
            &mut e,
            src_z,
            probe_z,
            &format!("up fuzz[{i}] src={src_z} probe={probe_z}"),
        );
    }
}

/// `0x670db0`'s colour block.
#[test]
fn create_doodad_color_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    color_block_difftest(&mut e, 0x670ec6, 0x670fc4, "create_doodad");
}

/// `0x69e280`'s colour block.
#[test]
fn env_update_color_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    color_block_difftest(&mut e, 0x69e384, 0x69e483, "env_update");
}

/// `0x694bc0` from `0x694c8b` to `0x694d55`: the record through `[ebp+0xc]`, the tile origin
/// through `[ebp+8]`; the definition at `esi` and the angles at `[ebp-0x14]` are read back.
#[test]
fn mddf_create_xform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40); // ECX (via [ebp-4]): MDDF record
    let origin = e.alloc(0x20); // EDX (via [ebp+8]): tile origin float3
    let obj = e.alloc(0x400); // ESI: object (pos@+0x24, scale@+0x30, + interior copies)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, r: &MddfRecord, org: [f32; 3], ctx: &str| {
        // MDDF record floats at the binary's offsets.
        e.write_u32(rec + 0x8, r.y.to_bits());
        e.write_u32(rec + 0xc, r.z.to_bits());
        e.write_u32(rec + 0x10, r.x.to_bits());
        e.write_u32(rec + 0x14, r.rot_x_deg.to_bits());
        e.write_u32(rec + 0x18, r.rot_y_deg.to_bits());
        e.write_u32(rec + 0x1c, r.rot_z_deg.to_bits());
        e.write_u32(rec + 0x20, u32::from(r.scale_u16)); // movzx word [ecx+0x20]
        for (k, &f) in org.iter().enumerate() {
            e.write_u32(origin + 4 * k as u64, f.to_bits());
        }
        // junk-seed every captured slot so a non-write would diverge.
        for off in [0x24u64, 0x28, 0x2c, 0x30] {
            e.write_u32(obj + off, 0xdead_beef);
        }
        for off in [0x14u64, 0x10, 0xc] {
            e.write_u32(ebp - off, 0xdead_beef);
        }
        e.write_u32(ebp - 0x4, rec as u32); // [ebp-4] = MDDF record (→ ecx)
        e.write_u32(ebp + 0x8, origin as u32); // [ebp+8] = tile origin (→ edx)
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x694c8b, 0x694d55);

        let bin = MddfXform {
            pos: [
                e.read_f32(obj + 0x24),
                e.read_f32(obj + 0x28),
                e.read_f32(obj + 0x2c),
            ],
            scale: e.read_f32(obj + 0x30),
            rot_z: e.read_f32(ebp - 0x14),
            rot_x: e.read_f32(ebp - 0x10),
            rot_y: e.read_f32(ebp - 0xc),
        };
        let ours = mddf_create_xform(r, org);
        // bit-exact on every f32 (NaN-pair-equal not needed: inputs are finite placements).
        assert_eq!(
            bin.pos[0].to_bits(),
            ours.pos[0].to_bits(),
            "{ctx} pos.x: {:#010x} vs {:#010x}",
            bin.pos[0].to_bits(),
            ours.pos[0].to_bits()
        );
        assert_eq!(
            bin.pos[1].to_bits(),
            ours.pos[1].to_bits(),
            "{ctx} pos.y: {:#010x} vs {:#010x}",
            bin.pos[1].to_bits(),
            ours.pos[1].to_bits()
        );
        assert_eq!(
            bin.pos[2].to_bits(),
            ours.pos[2].to_bits(),
            "{ctx} pos.z: {:#010x} vs {:#010x}",
            bin.pos[2].to_bits(),
            ours.pos[2].to_bits()
        );
        assert_eq!(
            bin.scale.to_bits(),
            ours.scale.to_bits(),
            "{ctx} scale: {:#010x} vs {:#010x}",
            bin.scale.to_bits(),
            ours.scale.to_bits()
        );
        assert_eq!(
            bin.rot_z.to_bits(),
            ours.rot_z.to_bits(),
            "{ctx} rot_z: {:#010x} vs {:#010x}",
            bin.rot_z.to_bits(),
            ours.rot_z.to_bits()
        );
        assert_eq!(
            bin.rot_x.to_bits(),
            ours.rot_x.to_bits(),
            "{ctx} rot_x: {:#010x} vs {:#010x}",
            bin.rot_x.to_bits(),
            ours.rot_x.to_bits()
        );
        assert_eq!(
            bin.rot_y.to_bits(),
            ours.rot_y.to_bits(),
            "{ctx} rot_y: {:#010x} vs {:#010x}",
            bin.rot_y.to_bits(),
            ours.rot_y.to_bits()
        );
    };

    // exact probes: zero placement, simple coords + origin, exact-degree rotations, scale steps.
    let zero = MddfRecord {
        y: 0.0,
        z: 0.0,
        x: 0.0,
        rot_x_deg: 0.0,
        rot_y_deg: 0.0,
        rot_z_deg: 0.0,
        scale_u16: 0,
    };
    run_case(&mut e, &zero, [0.0, 0.0, 0.0], "all-zero");
    run_case(
        &mut e,
        &MddfRecord {
            y: 2.0,
            z: 3.0,
            x: 1.0,
            rot_x_deg: 90.0,
            rot_y_deg: 180.0,
            rot_z_deg: 45.0,
            scale_u16: 1024,
        },
        [100.0, 200.0, 300.0],
        "pos=(-1,-2,3)+(100,200,300); scale=1.0; rot 90/180/45",
    );
    run_case(
        &mut e,
        &MddfRecord {
            y: -5.5,
            z: 7.25,
            x: -3.0,
            rot_x_deg: 360.0,
            rot_y_deg: 0.0,
            rot_z_deg: -90.0,
            scale_u16: 0xffff,
        },
        [17066.666, 17066.666, 0.0],
        "neg coords, max scale, tile-origin near grid max",
    );
    // scale-step probes: u16 → u16/1024, the exact division grid.
    for &u in &[1u16, 2, 512, 1023, 1025, 2048, 0x8000, 0xffff] {
        run_case(
            &mut e,
            &MddfRecord {
                scale_u16: u,
                ..zero
            },
            [0.0, 0.0, 0.0],
            &format!("scale u16={u}"),
        );
    }

    let mut s = 0x6d64_6466u32; // "mddf"
    for i in 0..6000u32 {
        let r = MddfRecord {
            y: fuzz_f32(&mut s),
            z: fuzz_f32(&mut s),
            x: fuzz_f32(&mut s),
            rot_x_deg: frand(&mut s, -720.0, 720.0),
            rot_y_deg: frand(&mut s, -720.0, 720.0),
            rot_z_deg: frand(&mut s, -720.0, 720.0),
            scale_u16: (next(&mut s) >> 8) as u16,
        };
        let org = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        run_case(&mut e, &r, org, &format!("mddf fuzz[{i}]"));
    }
}
