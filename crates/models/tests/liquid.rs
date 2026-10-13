//! A group's liquid grid: `CMapObjGroup::LiquidHeight` (`0x6b9f10`) and
//! `CMapObjGroup::LiquidTileFlags` (`0x6b9e50`) run whole, `InitMapObjGroupInvTileSize`
//! (`0x6bcf90`) whole, and the arithmetic of `CMapObjGroup::BuildLiquidVertices` (`0x6ba9b0`),
//! `CMapObjGroup::RayHitLiquid` (`0x6baa70`), `CMapObjGroup::CollectLiquidSegment` (`0x6bb170`)
//! and `CMapObjGroup::CollectLiquidVolume` (`0x6bb6b0`) as slices. A stretch run in an island is
//! the exe's own bytes, checked, copied between a load and a store.
//!
//! The grid step `0xce26dc` is zero until its initializer runs, so each test writes it first. A NaN
//! or infinite coordinate is not run through the grid: the CRT `floor` (`0x73ff3f`) takes its
//! exception path for them, which the harness does not map.

#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::liquid::{
    inv_tile_size, liquid_advance, liquid_cell_scale, liquid_sub, mliq_grid_coord, mliq_hit_scale,
    mliq_liquid_height, mliq_ray_normalize, mliq_straddle_bit_hi, mliq_straddle_bit_lo,
    mliq_straddle_delta, mliq_translate, MliqHeight, LIQUID_TILE_SIZE, MLIQ_GRID_STEP,
    MLIQ_HEIGHT_EPS, MLIQ_STRADDLE_BIAS,
};
use common::{
    assert_f32, assert_f64, frand, fuzz_wide, island_bytes_match, island_f32, next, read_f64,
    SLICE_ESP,
};

/// The grid step global.
const GRID_STEP_GLOBAL: u64 = 0x00ce_26dc;

fn write_grid_step(e: &mut Emu) {
    e.write_u32(GRID_STEP_GLOBAL, MLIQ_GRID_STEP.to_bits());
}

/// A coordinate `base + g/step` whose grid coordinate is exactly `g`, searched over nearby `f32`s.
fn coord_with_grid(base: f32, g: f32) -> f32 {
    let guess = (f64::from(base) + f64::from(g) / f64::from(MLIQ_GRID_STEP)) as f32;
    for d in 0..64i64 {
        for c in [guess.to_bits() as i64 + d, guess.to_bits() as i64 - d] {
            let x = f32::from_bits(c as u32);
            if mliq_grid_coord(x, base, MLIQ_GRID_STEP).0 == g {
                return x;
            }
        }
    }
    panic!("no coordinate maps to {g}");
}

/// `0x6bcf90` whole: it stores the grid step.
#[test]
fn inv_tile_size_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.write_u32(GRID_STEP_GLOBAL, 0);
    e.call(0x6bcf90, 0, 0, &[]);
    assert_f32(e.read_f32(GRID_STEP_GLOBAL), inv_tile_size(), "0xce26dc");
    assert_eq!(inv_tile_size().to_bits(), MLIQ_GRID_STEP.to_bits());
}

/// The memory of a group with a liquid grid.
struct LiquidScene {
    this: u64,
    pos: u64,
    out_type: u64,
    out_height: u64,
    out_normal: u64,
    tile_base: u64,
    height_base: u64,
    cols: i32,
    rows: i32,
    row_stride: i32,
}

/// A group with a `cols`×`rows` grid: `(cols + 1)·(rows + 1)` vertex records and one tile byte a
/// cell.
fn build_liquid_scene(e: &mut Emu, cols: i32, rows: i32) -> LiquidScene {
    let this = e.alloc(0x200);
    let pos = e.alloc(0x10);
    let out_type = e.alloc(0x10);
    let out_height = e.alloc(0x10);
    let out_normal = e.alloc(0x10);
    let n_verts = ((cols + 1) * (rows + 1)) as u64;
    let height_base = e.alloc(n_verts * 8 + 0x20);
    let tile_base = e.alloc((cols * rows) as u64 + 0x20);
    let row_stride = cols + 1;
    e.write_u32(this + 0x100, cols as u32);
    e.write_u32(this + 0x104, rows as u32);
    e.write_u32(this + 0xf8, row_stride as u32);
    e.write_u32(this + 0x118, height_base as u32);
    e.write_u32(this + 0x11c, tile_base as u32);
    LiquidScene {
        this,
        pos,
        out_type,
        out_height,
        out_normal,
        tile_base,
        height_base,
        cols,
        rows,
        row_stride,
    }
}

/// `0x6b9f10` whole: `ecx` the group, then the point and the type, height and normal outputs.
/// Cases: the type override, interpolated tiles of every nibble, the height limits exactly and a
/// step either side, points west and south of the grid (where `floor` and truncation differ), a
/// grid coordinate of 2^32 (whose cell `__ftol` makes 0), and a NaN corner.
#[test]
fn mliq_liquid_height_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    write_grid_step(&mut e);

    let (cols, rows) = (4i32, 4i32);
    let n_verts = ((cols + 1) * (rows + 1)) as usize;
    let mut s = 0x6b9f_1010u32;
    let sc = build_liquid_scene(&mut e, cols, rows);
    let base = [10.0f32, 20.0f32];
    e.write_u32(sc.this + 0x108, base[0].to_bits());
    e.write_u32(sc.this + 0x10c, base[1].to_bits());

    let run = |e: &mut Emu,
               type_override: u32,
               pos: [f32; 3],
               heights: &[(f32, f32)],
               tiles: &[u8],
               ctx: &str| {
        e.write_u32(sc.this + 0x38, type_override);
        e.write_u32(sc.this + 0x10, 0x1000);
        for (i, &(a, b)) in heights.iter().enumerate() {
            e.write_u32(sc.height_base + (i as u64) * 8, a.to_bits());
            e.write_u32(sc.height_base + (i as u64) * 8 + 4, b.to_bits());
        }
        e.write(sc.tile_base, tiles);
        for (i, &c) in pos.iter().enumerate() {
            e.write_u32(sc.pos + (i as u64) * 4, c.to_bits());
        }
        for va in [sc.out_type, sc.out_height, sc.out_normal] {
            e.write_u32(va, 0xdead_beef);
        }
        let ret = e.call(
            0x6b9f10,
            sc.this as u32,
            0,
            &[
                sc.pos as u32,
                sc.out_type as u32,
                sc.out_height as u32,
                sc.out_normal as u32,
            ],
        );
        let hvec: Vec<f32> = (0..n_verts * 2)
            .map(|i| e.read_f32(sc.height_base + (i as u64) * 4))
            .collect();
        let ours: MliqHeight = mliq_liquid_height(
            type_override,
            pos,
            base,
            MLIQ_GRID_STEP,
            sc.cols,
            sc.rows,
            sc.cols,
            tiles,
            sc.row_stride,
            &hvec,
            MLIQ_HEIGHT_EPS,
        );
        let bin_hit = (ret & 0xff) != 0;
        assert_eq!(bin_hit, ours.hit, "{ctx}: hit (eax={ret:#x})");
        if ours.hit {
            assert_eq!(e.read_u32(sc.out_type), ours.liquid_type, "{ctx}: type");
            assert_f32(
                e.read_f32(sc.out_height),
                ours.height,
                &format!("{ctx}: height"),
            );
        }
        ours.hit
    };

    let flat_tiles = vec![0u8; (cols * rows) as usize];
    let zero_heights: Vec<(f32, f32)> = vec![(0.0, 0.0); n_verts];
    for ty in [0u32, 1, 2, 3, 7, 0xe, 0x10] {
        run(
            &mut e,
            ty,
            [12.0, 22.0, 5.0],
            &zero_heights,
            &flat_tiles,
            &format!("override {ty}"),
        );
    }

    let world_step = 1.0f32 / MLIQ_GRID_STEP;
    for kind in 0..3u32 {
        for _ in 0..200 {
            let h: Vec<(f32, f32)> = (0..n_verts)
                .map(|_| (frand(&mut s, -5.0, 5.0), frand(&mut s, -50.0, 50.0)))
                .collect();
            let t: Vec<u8> = (0..cols * rows)
                .map(|_| match next(&mut s) % 5 {
                    0 => 0x00,
                    1 => 0x02,
                    2 => 0x03,
                    3 => 0x0f,
                    _ => 0x01,
                })
                .collect();
            let gx = frand(&mut s, 0.0, (cols as f32) - 0.01);
            let gy = frand(&mut s, 0.0, (rows as f32) - 0.01);
            let px = base[0] + gx * world_step;
            let py = base[1] + gy * world_step;
            let pz = match kind {
                0 => frand(&mut s, -60.0, 60.0),
                1 => frand(&mut s, -5.0, 5.0),
                _ => frand(&mut s, -200.0, 200.0),
            };
            run(&mut e, 0xf, [px, py, pz], &h, &t, "grid");
        }
    }

    // A flat surface, so h is exact: z at 0.01 + h and at h, and a step or two either side.
    let flat_h = 3.5f32;
    let h_flat = vec![(0.0, flat_h); n_verts];
    let px = base[0] + 1.5 * world_step;
    let py = base[1] + 1.5 * world_step;
    let step = |v: f32, d: i32| f32::from_bits((v.to_bits() as i64 + d as i64) as u32);
    let eps_plus_h = (f64::from(MLIQ_HEIGHT_EPS) + f64::from(flat_h)) as f32;
    for d in -2..=2 {
        let ctx = format!("0.01 + h, step {d}");
        run(
            &mut e,
            0xf,
            [px, py, step(eps_plus_h, d)],
            &h_flat,
            &flat_tiles,
            &ctx,
        );
    }
    for nib in [0x02u8, 0x03] {
        let t = vec![nib; (cols * rows) as usize];
        for d in -2..=2 {
            let ctx = format!("h, nibble {nib}, step {d}");
            run(&mut e, 0xf, [px, py, step(flat_h, d)], &h_flat, &t, &ctx);
        }
    }

    // Nibble 1 misses whatever the height.
    let t_one = vec![0x01u8; (cols * rows) as usize];
    run(&mut e, 0xf, [px, py, -100.0], &h_flat, &t_one, "nibble 1");

    // The nibbles the shipped files carry above 3, and the high bits: the type is `nibble & 3`.
    for tile in [
        0x04u8, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x44, 0x46, 0x47, 0x84,
        0xc4,
    ] {
        let t = vec![tile; (cols * rows) as usize];
        let ctx = format!("tile {tile:#04x}");
        let hit = run(&mut e, 0xf, [px, py, -100.0], &h_flat, &t, &ctx);
        let n = u32::from(tile & 0xf);
        assert_eq!(hit, n & 3 != 1, "{ctx}: hit");
        if hit {
            assert_eq!(e.read_u32(sc.out_type), n & 3, "{ctx}: type");
        }
    }

    // West and south of the grid: `floor` gives −1 and misses where truncation would give 0.
    for (gx, gy) in [
        (-0.5f32, 1.5f32),
        (-1.5, 1.5),
        (-0.001, 1.5),
        (1.5, -0.5),
        (-2.5, -2.5),
        (-0.5, -0.5),
    ] {
        let p = [base[0] + gx * world_step, base[1] + gy * world_step, 0.0];
        run(
            &mut e,
            0xf,
            p,
            &h_flat,
            &flat_tiles,
            &format!("west/south {gx} {gy}"),
        );
    }

    // A grid coordinate of ±2^32: `__ftol` keeps the low dword, cell 0, and the point hits.
    for g in [4_294_967_296.0f32, -4_294_967_296.0] {
        let far = coord_with_grid(base[0], g);
        let p = [far, py, -100.0];
        let hit = run(&mut e, 0xf, p, &h_flat, &flat_tiles, &format!("grid x {g}"));
        assert!(hit, "grid x {g}: the cell wraps to 0");
        let far = coord_with_grid(base[1], g);
        run(
            &mut e,
            0xf,
            [px, far, -100.0],
            &h_flat,
            &flat_tiles,
            &format!("grid y {g}"),
        );
    }

    // A NaN corner makes h NaN, and the ordered compare misses.
    let mut h_nan = vec![(0.0, 1.0); n_verts];
    h_nan[7] = (0.0, f32::NAN);
    run(
        &mut e,
        0xf,
        [px, py, 0.0],
        &h_nan,
        &flat_tiles,
        "nan corner",
    );
    run(
        &mut e,
        0xf,
        [px, py, f32::NAN],
        &h_flat,
        &flat_tiles,
        "nan z",
    );
}

/// `mliq_grid_coord` alone, without the exe: the cell is `floor`, not truncation, and the fraction
/// is taken from the stored coordinate.
#[test]
fn mliq_grid_coord_selfcheck() {
    let gs = MLIQ_GRID_STEP;
    for (coord, base) in [
        (0.0f32, 0.0f32),
        (10.0, 5.0),
        (-3.0, 2.0),
        (2.5, 3.0),
        (100.0, 0.0),
        (4.166666, 0.0),
    ] {
        let (g, cell, frac) = mliq_grid_coord(coord, base, gs);
        assert_eq!(
            g,
            (f64::from(gs) * (f64::from(coord) - f64::from(base))) as f32
        );
        assert_eq!(cell, g.floor() as i32);
        assert_eq!(frac, (f64::from(g) - f64::from(cell)) as f32);
    }
}

/// `0x6baa70` from `0x6baa9f` to `0x6bab0d`: the ray's transformed ends at `[ebp-0x98]` and
/// `[ebp-0x8c]` relative to the group's base (`ebx`), stored at `[ebp-0xf0]`, `[ebp-0xec]`,
/// `[ebp-0xe4]` and `[ebp-0xe0]`.
#[test]
fn mliq_ray_grid_sub_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x140;

    let run = |e: &mut Emu, start: [f32; 2], end: [f32; 2], base: [f32; 2], ctx: &str| {
        e.write_u32(ebp - 0x98, start[0].to_bits());
        e.write_u32(ebp - 0x94, start[1].to_bits());
        e.write_u32(ebp - 0x90, 0);
        e.write_u32(ebp - 0x8c, end[0].to_bits());
        e.write_u32(ebp - 0x88, end[1].to_bits());
        e.write_u32(ebp - 0x84, 0);
        e.write_u32(this + 0x108, base[0].to_bits());
        e.write_u32(this + 0x10c, base[1].to_bits());
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6baa9f, 0x6bab0d);
        let bin = [0xf0, 0xec, 0xe4, 0xe0].map(|d| e.read_f32(ebp - d));
        let ours = [
            liquid_sub(start[0], base[0]),
            liquid_sub(start[1], base[1]),
            liquid_sub(end[0], base[0]),
            liquid_sub(end[1], base[1]),
        ];
        for i in 0..4 {
            assert_f32(bin[i], ours[i], &format!("{ctx}: [{i}]"));
        }
    };

    run(&mut e, [0.0, 0.0], [0.0, 0.0], [0.0, 0.0], "zeros");
    run(&mut e, [5.0, 5.0], [5.0, 5.0], [5.0, 5.0], "equal");
    run(
        &mut e,
        [100.0, 200.0],
        [300.0, 400.0],
        [50.0, 60.0],
        "spread",
    );
    run(
        &mut e,
        [f32::from_bits(1), 0.0],
        [0.0, 0.0],
        [0.0, 0.0],
        "subnormal",
    );
    let mut s = 0x6baa_9f9fu32;
    for i in 0..3000u32 {
        let start = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        let end = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        let base = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        run(&mut e, start, end, base, &format!("fuzz[{i}]"));
    }
}

/// `0x6baa70` from `0x6bab12` to `0x6bab51`: the four coordinates at `[ebp-0xfc]`, `[ebp-0xf8]`,
/// `[ebp-0x108]` and `[ebp-0x104]` times the grid step; the first is read as the `f64` the slice
/// stores at `[esp]`, the others from `[ebp-0x54]`, `[ebp-0x1c]` and `[ebp-0x18]`.
#[test]
fn mliq_ray_grid_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    write_grid_step(&mut e);
    let frame = e.alloc(0x300);
    let ebp = frame + 0x180;

    let run = |e: &mut Emu, coords: [f32; 4], ctx: &str| {
        for (i, d) in [0xfc, 0xf8, 0x108, 0x104].into_iter().enumerate() {
            e.write_u32(ebp - d, coords[i].to_bits());
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bab12, 0x6bab51);
        let (o0, rest) = liquid_cell_scale(MLIQ_GRID_STEP, coords);
        assert_f64(read_f64(e, SLICE_ESP - 8), o0, &format!("{ctx}: first"));
        for (i, d) in [0x54, 0x1c, 0x18].into_iter().enumerate() {
            assert_f32(e.read_f32(ebp - d), rest[i], &format!("{ctx}: [{}]", i + 1));
        }
    };

    run(&mut e, [0.0; 4], "zeros");
    run(&mut e, [1.0; 4], "ones");
    run(&mut e, [100.0, 200.0, 300.0, 400.0], "spread");
    let inv = 1.0f32 / MLIQ_GRID_STEP;
    run(&mut e, [inv, 2.0 * inv, 3.0 * inv, inv * 0.5], "cell edges");
    run(
        &mut e,
        [f32::NAN, f32::INFINITY, -0.0, f32::NEG_INFINITY],
        "nan and infinities",
    );
    let mut s = 0x6bab_1212u32;
    for i in 0..3000u32 {
        let c = [0; 4].map(|_| frand(&mut s, -500.0, 500.0));
        run(&mut e, c, &format!("fuzz[{i}]"));
    }
}

/// `0x6baa70` from `0x6babd0` to `0x6bac4e`: the ray at `esi` (start, then end), its inverse
/// length read from `[ebp-0x44]` and its unit direction from `[ebp-0x34]`. A ray of length 0
/// divides by zero, which the slice does unguarded.
#[test]
fn mliq_ray_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ray = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x140;

    let run = |e: &mut Emu, start: [f32; 3], end: [f32; 3], ctx: &str| {
        for i in 0..3 {
            e.write_u32(ray + (i as u64) * 4, start[i].to_bits());
            e.write_u32(ray + 0xc + (i as u64) * 4, end[i].to_bits());
        }
        e.set_reg(RegisterX86::ESI, ray);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6babd0, 0x6bac4e);
        let (inv, unit) = mliq_ray_normalize(start, end);
        assert_f32(
            e.read_f32(ebp - 0x44),
            inv,
            &format!("{ctx}: inverse length"),
        );
        for i in 0..3 {
            let got = e.read_f32(ebp - 0x34 + 4 * i as u64);
            assert_f32(got, unit[i], &format!("{ctx}: unit[{i}]"));
        }
    };

    run(&mut e, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "unit x");
    run(&mut e, [0.0, 0.0, 0.0], [3.0, 4.0, 0.0], "3-4-5");
    run(&mut e, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], "length 0");
    run(&mut e, [-10.0, 5.0, -2.0], [20.0, -8.0, 6.0], "spread");
    run(&mut e, [0.0, 0.0, 0.0], [f32::NAN, 1.0, 0.0], "nan");
    let mut s = 0x6bab_d0d0u32;
    for i in 0..3000u32 {
        let start = [0; 3].map(|_| frand(&mut s, -1000.0, 1000.0));
        let end = [0; 3].map(|_| frand(&mut s, -1000.0, 1000.0));
        run(&mut e, start, end, &format!("fuzz[{i}]"));
    }
}

/// `0x6baa70`'s corner test: `0x6bad30` to `0x6bad48` stores `(h − lo) + bias`, `0x6bad54` to
/// `0x6bad69` stores `(hi − h) + bias`, both at `[ebp+0xc]`; `0x6bad30` to `0x6bad7e` runs the
/// iteration and ORs the two sign bits into the mask at `ecx`. The deltas are probed where the bias
/// makes them exactly 0.
#[test]
fn mliq_straddle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x300);
    let ebp = frame + 0x180;
    let harr = e.alloc(0x40);
    let mask = e.alloc(0x40);

    let d0_case = |e: &mut Emu, h: f32, lo: f32, ctx: &str| {
        e.write_u32(harr + 8, h.to_bits());
        e.write_u32(ebp - 0x90, lo.to_bits());
        e.set_reg(RegisterX86::EAX, harr);
        e.set_reg(RegisterX86::ECX, mask + 0x10);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bad30, 0x6bad48);
        let ours = mliq_straddle_delta(h, lo, MLIQ_STRADDLE_BIAS);
        assert_f32(e.read_f32(ebp + 0xc), ours, &format!("{ctx}: below"));
    };
    let d1_case = |e: &mut Emu, h: f32, hi: f32, ctx: &str| {
        e.write_u32(harr + 8, h.to_bits());
        e.write_u32(ebp - 0x84, hi.to_bits());
        e.set_reg(RegisterX86::EAX, harr + 0xc);
        e.set_reg(RegisterX86::ECX, mask + 0x10);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bad54, 0x6bad69);
        let ours = mliq_straddle_delta(hi, h, MLIQ_STRADDLE_BIAS);
        assert_f32(e.read_f32(ebp + 0xc), ours, &format!("{ctx}: above"));
    };
    let mask_case = |e: &mut Emu, h: f32, lo: f32, hi: f32, ctx: &str| {
        e.write_u32(harr + 8, h.to_bits());
        e.write_u32(ebp - 0x90, lo.to_bits());
        e.write_u32(ebp - 0x84, hi.to_bits());
        e.write_u32(ebp - 0x8, 0);
        let slot = mask + 0x10;
        e.write_u32(slot, 0xffff_ffff);
        e.set_reg(RegisterX86::EAX, harr);
        e.set_reg(RegisterX86::ECX, slot);
        e.set_reg(RegisterX86::ESI, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bad30, 0x6bad7e);
        let d0 = mliq_straddle_delta(h, lo, MLIQ_STRADDLE_BIAS);
        let d1 = mliq_straddle_delta(hi, h, MLIQ_STRADDLE_BIAS);
        let ours = mliq_straddle_bit_lo(d0) | mliq_straddle_bit_hi(d1);
        assert_eq!(e.read_u32(slot), ours, "{ctx}: mask (d0={d0} d1={d1})");
    };

    let bias = MLIQ_STRADDLE_BIAS;
    let probes = [-bias, 0.0, bias, 1.0, -1.0, 1e-30, -1e-30, f32::NAN];
    for &h in &probes {
        for &r in &probes {
            d0_case(&mut e, h, r, &format!("h={h} r={r}"));
            d1_case(&mut e, h, r, &format!("h={h} r={r}"));
            mask_case(&mut e, h, r, r * 0.5, &format!("h={h} r={r}"));
        }
    }
    d0_case(&mut e, 5.0, 5.0 + bias, "below at 0");
    d1_case(&mut e, 5.0 + bias, 5.0, "above at 0");
    let mut s = 0x6bad_3030u32;
    for i in 0..3000u32 {
        let h = frand(&mut s, -100.0, 100.0);
        let lo = frand(&mut s, -100.0, 100.0);
        let hi = frand(&mut s, -100.0, 100.0);
        d0_case(&mut e, h, lo, &format!("fuzz[{i}]"));
        d1_case(&mut e, h, hi, &format!("fuzz[{i}]"));
        mask_case(&mut e, h, lo, hi, &format!("fuzz[{i}]"));
    }
}

/// `0x6baa70` from `0x6baf5e` to `0x6baf6a` in an island: `fld [ebp-4]; fmul [ebp-0x44]; fst
/// [ebp-4]` with `fstp qword` appended, so both the stored `f32` and the unrounded product the
/// following compare reads are checked.
#[test]
fn mliq_hit_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x6baf5e;
    const LEN: usize = 0xc;
    let out_f64 = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let mut code = e.read(SRC, LEN);
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out_f64 as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let ecx_target = e.alloc(0x10);

    let run = |e: &mut Emu, t: f32, inv: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, t.to_bits());
        e.write_u32(ebp - 0x44, inv.to_bits());
        e.write_u32(ebp + 0x10, ecx_target as u32);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let (stored, live) = mliq_hit_scale(t, inv);
        assert_f32(e.read_f32(ebp - 0x4), stored, &format!("{ctx}: stored"));
        assert_f64(read_f64(e, out_f64), live, &format!("{ctx}: compared"));
    };

    run(&mut e, 0.0, 1.0, "zero");
    run(&mut e, 1.0, 1.0, "one");
    run(&mut e, 3.0, MLIQ_GRID_STEP, "times the grid step");
    run(
        &mut e,
        f32::from_bits(0x3fb0_0001),
        f32::from_bits(0x3f80_0001),
        "odd mantissas",
    );
    run(&mut e, f32::NAN, 1.0, "nan");
    let mut s = 0x6baf_5e5eu32;
    for i in 0..3000u32 {
        let t = frand(&mut s, -100.0, 100.0);
        let inv = frand(&mut s, 1e-4, 10.0);
        run(&mut e, t, inv, &format!("fuzz[{i}]"));
    }
}

/// `0x6bb170` from `0x6bb18d` to `0x6bb1df`: the volume's box at `[ebp+0xc]` relative to the
/// group's base (`ebx`); the four differences land at `[ebp-0x88]`, `[ebp-0x84]`, `[ebp-0x7c]` and
/// `[ebp-0x78]`, the two heights unchanged at `[ebp-0x80]` and `[ebp-0x74]`.
#[test]
fn mliq_seg_translate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vol = e.alloc(0x20);
    let this = e.alloc(0x200);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run = |e: &mut Emu, aabb: [f32; 6], base: [f32; 2], ctx: &str| {
        for (k, c) in aabb.iter().enumerate() {
            e.write_u32(vol + 4 * k as u64, c.to_bits());
        }
        e.write_u32(this + 0x108, base[0].to_bits());
        e.write_u32(this + 0x10c, base[1].to_bits());
        e.write_u32(ebp + 0xc, vol as u32);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bb18d, 0x6bb1df);
        let bin = [0x88, 0x84, 0x7c, 0x78].map(|d| e.read_f32(ebp - d));
        let ours = [
            liquid_sub(aabb[0], base[0]),
            liquid_sub(aabb[1], base[1]),
            liquid_sub(aabb[3], base[0]),
            liquid_sub(aabb[4], base[1]),
        ];
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("{ctx}: [{k}]"));
        }
        assert_f32(e.read_f32(ebp - 0x80), aabb[2], &format!("{ctx}: z0"));
        assert_f32(e.read_f32(ebp - 0x74), aabb[5], &format!("{ctx}: z1"));
    };

    run(
        &mut e,
        [10.0, 20.0, 5.0, 30.0, 40.0, 15.0],
        [10.0, 20.0],
        "exact",
    );
    let nan = f32::NAN;
    run(
        &mut e,
        [nan, 20.0, nan, 30.0, nan, 15.0],
        [10.0, nan],
        "nan",
    );
    let mut s = 0x6bb1_7070u32;
    for i in 0..4000u32 {
        let aabb = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -500.0, 500.0),
        ];
        let base = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        run(&mut e, aabb, base, &format!("fuzz[{i}]"));
    }
}

/// `0x6bb170` from `0x6bb1e4` to `0x6bb223`: the transformed coordinates at `[ebp-0xac]`,
/// `[ebp-0xa8]`, `[ebp-0xb8]` and `[ebp-0xb4]` times the grid step; the first is read as the `f64`
/// at `[esp]`, the others from `[ebp-0x44]`, `[ebp-0x38]` and `[ebp-0x34]`.
#[test]
fn mliq_seg_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    write_grid_step(&mut e);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x180;

    let run = |e: &mut Emu, c: [f32; 4], ctx: &str| {
        for (i, d) in [0xac, 0xa8, 0xb8, 0xb4].into_iter().enumerate() {
            e.write_u32(ebp - d, c[i].to_bits());
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bb1e4, 0x6bb223);
        let (first, rest) = liquid_cell_scale(MLIQ_GRID_STEP, c);
        assert_f64(read_f64(e, SLICE_ESP - 8), first, &format!("{ctx}: first"));
        for (i, d) in [0x44, 0x38, 0x34].into_iter().enumerate() {
            assert_f32(e.read_f32(ebp - d), rest[i], &format!("{ctx}: [{}]", i + 1));
        }
    };

    run(&mut e, [1.0, 2.0, 3.0, 4.0], "exact");
    run(&mut e, [0.0, -0.0, 4.1667, -4.1667], "zeros");
    run(
        &mut e,
        [f32::NAN, f32::INFINITY, 1.0, -1.0],
        "nan and infinity",
    );
    let mut s = 0x6bb1_e4e4u32;
    for i in 0..4000u32 {
        let c = [0; 4].map(|_| frand(&mut s, -4000.0, 4000.0));
        run(&mut e, c, &format!("fuzz[{i}]"));
    }
}

/// `0x6bb170`'s two corner bounds, each `(z − z') + bias` stored at `[ebp-0x2c]`: `0x6bb353` to
/// `0x6bb36b` with `z = [eax+8]` and `z' = [esi+8]` (`esi` from `[ebp+0xc]`), and `0x6bb377` to
/// `0x6bb386` with `z = [esi+0x14]` and `z' = [eax-4]`.
#[test]
fn mliq_seg_tri_bound_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let eax_buf = e.alloc(0x40) + 0x10;
    let param3 = e.alloc(0x40);
    let ecx_buf = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_lower = |e: &mut Emu, z: f32, zp: f32, ctx: &str| {
        e.write_u32(eax_buf + 8, z.to_bits());
        e.write_u32(param3 + 8, zp.to_bits());
        e.write_u32(ebp + 0xc, param3 as u32);
        e.set_reg(RegisterX86::EAX, eax_buf);
        e.set_reg(RegisterX86::ECX, ecx_buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bb353, 0x6bb36b);
        let ours = mliq_straddle_delta(z, zp, MLIQ_STRADDLE_BIAS);
        assert_f32(e.read_f32(ebp - 0x2c), ours, &format!("{ctx}: lower"));
    };
    let run_upper = |e: &mut Emu, z: f32, zp: f32, ctx: &str| {
        e.write_u32(param3 + 0x14, z.to_bits());
        e.write_u32(eax_buf - 4, zp.to_bits());
        e.set_reg(RegisterX86::ESI, param3);
        e.set_reg(RegisterX86::EAX, eax_buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bb377, 0x6bb386);
        let ours = mliq_straddle_delta(z, zp, MLIQ_STRADDLE_BIAS);
        assert_f32(e.read_f32(ebp - 0x2c), ours, &format!("{ctx}: upper"));
    };

    let bias = MLIQ_STRADDLE_BIAS;
    for (z, zp) in [
        (0.0f32, 0.0f32),
        (5.0, 5.0),
        (10.0, -10.0),
        (-3.5, 7.25),
        (100.0, 0.0),
        (5.0, 5.0 + bias),
        (f32::NAN, 1.0),
    ] {
        run_lower(&mut e, z, zp, &format!("{z} {zp}"));
        run_upper(&mut e, z, zp, &format!("{z} {zp}"));
    }
    let mut s = 0x6bb3_5959u32;
    for i in 0..4000u32 {
        let z = frand(&mut s, -1000.0, 1000.0);
        let zp = frand(&mut s, -1000.0, 1000.0);
        run_lower(&mut e, z, zp, &format!("fuzz[{i}]"));
        run_upper(&mut e, z, zp, &format!("fuzz[{i}]"));
    }
}

/// `0x6ba9b0`'s x step at `0x6baa1c`, `fadd dword [0x810d84]`, in an island after a load of the
/// running x.
#[test]
fn mliq_vertex_advance_x_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x6baa1c;
    let seed = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let body = e.read(SRC, 6);
    let end = island_f32(&mut e, isl, seed, &body, out);
    island_bytes_match(&e, isl + 6, SRC, 6);

    let run = |e: &mut Emu, r: f32, ctx: &str| {
        e.write_u32(seed, r.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let ours = liquid_advance(r, LIQUID_TILE_SIZE);
        assert_f32(e.read_f32(out), ours, ctx);
    };

    for r in [
        0.0f32,
        10.0,
        -10.0,
        4.166_666_5,
        100.0,
        -1024.5,
        1e6,
        -1e-6,
        f32::NAN,
    ] {
        run(&mut e, r, &format!("x {r}"));
    }
    let mut s = 0x6baa_1c1cu32;
    for i in 0..3000u32 {
        run(
            &mut e,
            frand(&mut s, -4000.0, 4000.0),
            &format!("fuzz[{i}]"),
        );
    }
}

/// `0x6ba9b0`'s y step at `0x6baa4d`, `fadd dword [ebp-4]`, in an island after a load of the tile
/// size `0x810d84`.
#[test]
fn mliq_vertex_advance_y_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x6baa4d;
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let body = e.read(SRC, 3);
    let end = island_f32(&mut e, isl, 0x0081_0d84, &body, out);
    island_bytes_match(&e, isl + 6, SRC, 3);

    let run = |e: &mut Emu, r: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, r.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let ours = liquid_advance(LIQUID_TILE_SIZE, r);
        assert_f32(e.read_f32(out), ours, ctx);
    };

    for r in [
        0.0f32,
        20.0,
        -20.0,
        4.166_666_5,
        200.0,
        -2048.5,
        1e6,
        -1e-6,
        f32::NAN,
    ] {
        run(&mut e, r, &format!("y {r}"));
    }
    let mut s = 0x6baa_4d4du32;
    for i in 0..3000u32 {
        run(
            &mut e,
            frand(&mut s, -4000.0, 4000.0),
            &format!("fuzz[{i}]"),
        );
    }
}

/// `0x6bb6b0`'s three negations, `fld [ebx+0x108..0x110]; fchs; fstp [ebp-0x18..-0x10]`, copied
/// into one island without the integer moves between them.
#[test]
fn mliq_translate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frags: [(u64, &[u8]); 9] = [
        (0x6bb6d0, &[0xd9, 0x83, 0x08, 0x01, 0x00, 0x00]),
        (0x6bb6d8, &[0xd9, 0xe0]),
        (0x6bb757, &[0xd9, 0x5d, 0xe8]),
        (0x6bb75a, &[0xd9, 0x83, 0x0c, 0x01, 0x00, 0x00]),
        (0x6bb76c, &[0xd9, 0xe0]),
        (0x6bb774, &[0xd9, 0x5d, 0xec]),
        (0x6bb77d, &[0xd9, 0x83, 0x10, 0x01, 0x00, 0x00]),
        (0x6bb78f, &[0xd9, 0xe0]),
        (0x6bb797, &[0xd9, 0x5d, 0xf0]),
    ];
    let mut code = Vec::new();
    for (va, want) in frags {
        let got = e.read(va, want.len());
        assert_eq!(got, want, "bytes at {va:#x}");
        code.extend_from_slice(&got);
    }
    let obj = e.alloc(0x200);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let island = e.alloc(0x40);
    e.write(island, &code);
    let end = island + code.len() as u64;

    let run = |e: &mut Emu, t: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(obj + 0x108 + 4 * k as u64, t[k].to_bits());
            e.write_u32(ebp - 0x18 + 4 * k as u64, 0xdead_beef);
        }
        e.set_reg(RegisterX86::EBX, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(island, end);
        let ours = mliq_translate(t);
        for k in 0..3 {
            let got = e.read_f32(ebp - 0x18 + 4 * k as u64);
            assert_f32(got, ours[k], &format!("{ctx}: [{k}]"));
        }
    };

    run(&mut e, [0.0, 0.0, 0.0], "zero");
    run(&mut e, [1.0, -2.0, 3.0], "mixed");
    run(&mut e, [-100.0, 200.0, -300.0], "mid");
    run(&mut e, [1.0e30, -1.0e30, 1.0e-30], "huge and tiny");
    run(&mut e, [f32::NAN, f32::INFINITY, -0.0], "nan, infinity, -0");
    let mut s = 0x6bb6_b0b0u32;
    for i in 0..5000u32 {
        let t = [fuzz_wide(&mut s), fuzz_wide(&mut s), fuzz_wide(&mut s)];
        run(&mut e, t, &format!("fuzz[{i}]"));
    }
}

/// `0x6bb6b0` from `0x6bb7d0` to `0x6bb80f`: the step `[0xce26dc]` (written per case) times
/// `[ebp-0x94]`, `[ebp-0x90]`, `[ebp-0xa0]` and `[ebp-0x9c]`; the first is read as the `f64` at
/// `[esp]`, the others from `[ebp-0x44]`, `[ebp-0x14]` and `[ebp-0x10]`.
#[test]
fn mliq_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run = |e: &mut Emu, c: f32, op: [f32; 4], ctx: &str| {
        e.write_u32(GRID_STEP_GLOBAL, c.to_bits());
        for (i, d) in [0x94, 0x90, 0xa0, 0x9c].into_iter().enumerate() {
            e.write_u32(ebp - d, op[i].to_bits());
        }
        for d in [0x44, 0x14, 0x10] {
            e.write_u32(ebp - d, 0xdead_beef);
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bb7d0, 0x6bb80f);
        let (p0, rest) = liquid_cell_scale(c, op);
        assert_f64(read_f64(e, SLICE_ESP - 8), p0, &format!("{ctx}: first"));
        for (i, d) in [0x44, 0x14, 0x10].into_iter().enumerate() {
            assert_f32(e.read_f32(ebp - d), rest[i], &format!("{ctx}: [{}]", i + 1));
        }
    };

    run(&mut e, 0.0, [1.0, 2.0, 3.0, 4.0], "step 0");
    run(&mut e, 1.0, [1.0, 2.0, 3.0, 4.0], "step 1");
    run(&mut e, 2.0, [0.5, 1.5, -2.5, 3.5], "step 2");
    run(&mut e, -1.0, [1.0, 2.0, 3.0, 4.0], "step -1");
    run(&mut e, 1.0e20, [1.0e20, 1.0, 1.0, 1.0], "first past f32");
    run(
        &mut e,
        1.0,
        [f32::NAN, f32::INFINITY, 1.0, 1.0],
        "nan and infinity",
    );
    let mut s = 0x6bb7_d0d0u32;
    for i in 0..6000u32 {
        let c = fuzz_wide(&mut s);
        let op = [0; 4].map(|_| fuzz_wide(&mut s));
        run(&mut e, c, op, &format!("fuzz[{i}] step={c}"));
    }
}
