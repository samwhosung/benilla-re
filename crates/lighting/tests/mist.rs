//! The ground grid kernels against the exe: the cell height `0x67c820`, the grid height `0x67ca00`,
//! the cell position of `0x67ca90`, the segment walk `0x67cb60`, and `0x67a990` PlacePuff.
//!
//! The grid object and its 5 by 5 window of 0x1008-byte tiles are laid out in the emulator. The
//! ground sampler `0x6b7070` is stubbed to return 0, so the adapter `0x67c7f0` answers `z - 200`
//! (the `[0x810008]` fallback), which the `terrain_height` closures reproduce. `0x67a990` runs
//! whole: the generator (`0x4531e0` and its inlined copies), the tilt matrix (`0x7bdd60`,
//! `0x7bcae0`), the grid virtual `0x67ca00` through the real table `0x810038`, and `0x6745d0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::mist::{
    cell_world_xy, mist_grid_raymarch, mist_grid_sample, mist_spawn, tile_cell_fill, MistSpawnOut,
    MistSpawnSource, Tile, Vec3, FLT_MAX_SENTINEL,
};
use benilla_re_random::Stream;
use common::{assert_f32, next, straddling_y};

/// A tile: 0x400 cells, then the tile coordinates.
const TILE_BYTES: u64 = 0x1008;
/// Tiles in the window.
const WINDOW: usize = 25;
/// The ground sampler the adapter calls.
const GROUND_SAMPLER: u64 = 0x6b7070;
/// `[0x810008]`, what the adapter subtracts from z when the sampler finds nothing.
const HEIGHT_DROP: f32 = 200.0;
/// `[0x80ff90]`, the tile scale of `0x67ca00`.
const TILE_SCALE_SAMPLE: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x80ff94]`, the cell scale of `0x67cb60`.
const TILE_SCALE_MARCH: f32 = f32::from_bits(0x3f75_c28f);

/// Allocate the grid and its 25 tiles once (the arena never frees) and write the tile pointers at
/// `+0x28`; returns the grid and the tiles.
fn alloc_grid(emu: &mut Emu) -> (u64, Vec<u64>) {
    let grid = emu.alloc(0x100);
    let mut tiles = Vec::with_capacity(WINDOW);
    for w in 0..WINDOW {
        let tile = emu.alloc(TILE_BYTES);
        emu.write_u32(grid + 0x28 + 4 * w as u64, tile as u32);
        tiles.push(tile);
    }
    (grid, tiles)
}

/// Write the grid's `+0x18`, `+0x1c` (tile origin) and `+0x20`, `+0x24` (cell origin).
fn write_grid_origins(emu: &mut Emu, grid: u64, cell_origin: (i32, i32), origin_cell: (i32, i32)) {
    emu.write_u32(grid + 0x18, cell_origin.0 as u32);
    emu.write_u32(grid + 0x1c, cell_origin.1 as u32);
    emu.write_u32(grid + 0x20, origin_cell.0 as u32);
    emu.write_u32(grid + 0x24, origin_cell.1 as u32);
}

/// Write each tile's cells and coordinates.
fn fill_tiles(
    emu: &mut Emu,
    tiles: &[u64],
    tile_coords: &[(i32, i32)],
    initial_cells: &dyn Fn(usize) -> [f32; 0x400],
) {
    for (w, &tile) in tiles.iter().enumerate() {
        let cells = initial_cells(w);
        for (c, v) in cells.iter().enumerate() {
            emu.write_u32(tile + 4 * c as u64, v.to_bits());
        }
        let (tx, ty) = tile_coords[w];
        emu.write_u32(tile + 0x1000, tx as u32);
        emu.write_u32(tile + 0x1004, ty as u32);
    }
}

/// Read a tile's cells back.
fn read_cells(emu: &Emu, tile: u64) -> [f32; 0x400] {
    let mut a = [0.0f32; 0x400];
    for (c, v) in a.iter_mut().enumerate() {
        *v = emu.read_f32(tile + 4 * c as u64);
    }
    a
}

/// The window as the transcription takes it, read from the emulator.
fn read_tiles(emu: &Emu, tiles: &[u64], tile_coords: &[(i32, i32)]) -> Vec<Tile> {
    tiles
        .iter()
        .zip(tile_coords)
        .map(|(&t, &coord)| Tile {
            coord,
            cells: read_cells(emu, t),
        })
        .collect()
}

/// The coordinates of window slot `w` for a grid whose first tile is `origin`.
fn window_coords(origin: (i32, i32)) -> Vec<(i32, i32)> {
    (0..WINDOW)
        .map(|w| (origin.0 + (w % 5) as i32, origin.1 + (w / 5) as i32))
        .collect()
}

/// `0x67ca00` whole, 600 cases over a positive and a negative tile origin (the tile is the floor,
/// not the truncation), filled and empty cells, points inside and outside the window, and the y
/// values where the product stored as f32 floors to a different tile than the wide product.
#[test]
fn mist_grid_sample_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(GROUND_SAMPLER, 0, 1);

    let world_p = emu.alloc(0x10);
    let (grid, tiles) = alloc_grid(&mut emu);
    let mut s = 0x6d69_7374u32;

    let run = |emu: &mut Emu,
               cell_origin: (i32, i32),
               prefill: bool,
               wx: f32,
               wy: f32,
               wz: f32,
               ctx: &str| {
        let origin_cell = (cell_origin.0 << 5, cell_origin.1 << 5);
        write_grid_origins(emu, grid, cell_origin, origin_cell);
        let tile_coords = window_coords(cell_origin);
        let init = |w: usize| -> [f32; 0x400] {
            let mut a = [FLT_MAX_SENTINEL; 0x400];
            if prefill {
                for (k, v) in a.iter_mut().enumerate() {
                    *v = (k as f32) * 0.5 - 100.0 + 1000.0 * w as f32;
                }
            }
            a
        };
        fill_tiles(emu, &tiles, &tile_coords, &init);
        emu.write_u32(world_p, wx.to_bits());
        emu.write_u32(world_p + 4, wy.to_bits());
        emu.write_u32(world_p + 8, wz.to_bits());

        let binary = emu.call_f32(0x67ca00, grid as u32, 0, &[world_p as u32]);

        let mut tiles_ours = read_tiles(emu, &tiles, &tile_coords);
        let terrain = |_x: f32, _y: f32| (f64::from(wz) - f64::from(HEIGHT_DROP)) as f32;
        let out = mist_grid_sample(cell_origin, (wx, wy), &mut tiles_ours, terrain);
        assert_f32(
            binary,
            out.value,
            &format!("sample ({ctx} world=({wx},{wy},{wz}))"),
        );
    };

    for case in 0..600 {
        let prefill = case % 3 == 0;
        let cell_origin = if case % 2 == 0 {
            (3i32, 7i32)
        } else {
            (-10i32, -10i32)
        };
        let pick = (next(&mut s) >> 8) % 6;
        let (wx, wy) = if pick < 4 {
            let c = ((next(&mut s) >> 8) % 5) as i32;
            let r = ((next(&mut s) >> 8) % 5) as i32;
            let tx = cell_origin.0 + c;
            let ty = cell_origin.1 + r;
            let cx = ((next(&mut s) >> 8) % 40) as f32;
            let cy = ((next(&mut s) >> 8) % 40) as f32;
            (
                tx as f32 * 33.33333 + cx * 1.0416666 + 0.2,
                ty as f32 * 33.33333 + cy * 1.0416666 + 0.2,
            )
        } else {
            let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
            ((m * 4000.0 - 1000.0) as f32, (m * 4000.0 - 1000.0) as f32)
        };
        let wz = f32::from_bits(next(&mut s) & 0x4f00_0000 | 0x4080_0000);
        run(
            &mut emu,
            cell_origin,
            prefill,
            wx,
            wy,
            wz,
            &format!("case {case} pick={pick}"),
        );
    }

    let probes = straddling_y(TILE_SCALE_SAMPLE);
    assert!(!probes.is_empty(), "no y straddles a tile boundary");
    for &(y, stored_floor) in probes.iter().take(40) {
        let origin = (3i32, stored_floor - 3);
        run(
            &mut emu,
            origin,
            true,
            5.0 * 33.3 + 4.0,
            y,
            6.0,
            "y rounded to f32 before the floor",
        );
    }
    run(&mut emu, (3, 3), true, -0.0, -0.0, 6.0, "negative zero");

    // Coordinates whose tile index passes `i32` and `i64`: `__ftol` keeps the low dword of its
    // qword, which is 0 past `i64` and the low 32 bits of the index in between. A grid origin at
    // that value puts the point in the window. NaN and the infinities do not reach `__ftol`: the C
    // runtime's `floor` (`0x73ff3f`) takes its error path first, which the emulator does not map.
    let wide: [(f32, f32); 6] = [
        (1.0e30, -1.0e30),
        (1.43e11, 5.0),
        (5.0, -1.43e11),
        (-1.0e12, 1.0e12),
        (1.0e19, 4.0e12),
        (3.0e38, -3.0e38),
    ];
    for &(wx, wy) in &wide {
        let ty_scaled = (f64::from(TILE_SCALE_SAMPLE) * f64::from(wy)) as f32;
        let low = |v: f64| benilla_re_cmath::x87::ftol(v.floor());
        let fx = low(f64::from(TILE_SCALE_SAMPLE) * f64::from(wx));
        let fy = low(f64::from(ty_scaled));
        for origin in [(0, 0), (fx.wrapping_sub(2), fy.wrapping_sub(1))] {
            if origin.0.unsigned_abs() > 1 << 24 || origin.1.unsigned_abs() > 1 << 24 {
                continue;
            }
            run(&mut emu, origin, true, wx, wy, 6.0, "wide world coordinate");
        }
    }
}

/// `0x67c820` whole on one tile, 800 cases: the points left of or below the tile (rejected), the
/// cells 0x20 and over (rejected), filled and empty cells, and NaN coordinates (not rejected: the
/// `fcomp; test ah,5; jnp` skips only a value below 0) and -0.
#[test]
fn tile_cell_fill_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(GROUND_SAMPLER, 0, 1);

    let tile = emu.alloc(TILE_BYTES);
    let world_p = emu.alloc(0x10);
    let mut s = 0x7469_6c65u32;

    let run = |emu: &mut Emu,
               tile_coord: (i32, i32),
               prefill: bool,
               wx: f32,
               wy: f32,
               wz: f32,
               ctx: &str| {
        let mut cells = [FLT_MAX_SENTINEL; 0x400];
        if prefill {
            for (k, v) in cells.iter_mut().enumerate() {
                *v = (k as f32) * 0.25 - 50.0;
            }
        }
        for (c, v) in cells.iter().enumerate() {
            emu.write_u32(tile + 4 * c as u64, v.to_bits());
        }
        emu.write_u32(tile + 0x1000, tile_coord.0 as u32);
        emu.write_u32(tile + 0x1004, tile_coord.1 as u32);
        emu.write_u32(world_p, wx.to_bits());
        emu.write_u32(world_p + 4, wy.to_bits());
        emu.write_u32(world_p + 8, wz.to_bits());

        let binary = emu.call_f32(0x67c820, tile as u32, 0, &[world_p as u32]);

        let mut our_cells = read_cells(emu, tile);
        let terrain = |_x: f32, _y: f32| (f64::from(wz) - f64::from(HEIGHT_DROP)) as f32;
        let ours = tile_cell_fill(&mut our_cells, tile_coord, (wx, wy), terrain);
        assert_f32(
            binary,
            ours,
            &format!("tile_fill ({ctx} coord={tile_coord:?} world=({wx},{wy}) prefill={prefill})"),
        );
        let after = read_cells(emu, tile);
        for k in 0..0x400 {
            assert_f32(after[k], our_cells[k], &format!("cell {k} ({ctx})"));
        }
    };

    for case in 0..800 {
        let tile_coord = (
            ((next(&mut s) >> 8) % 200) as i32 - 100,
            ((next(&mut s) >> 8) % 200) as i32 - 100,
        );
        let base_x = tile_coord.0 as f32 * 33.33333;
        let base_y = tile_coord.1 as f32 * 33.33333;
        let off = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 50.0 - 5.0) as f32;
        let off2 = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 50.0 - 5.0) as f32;
        let wz = f32::from_bits(next(&mut s) & 0x4f00_0000 | 0x40a0_0000);
        run(
            &mut emu,
            tile_coord,
            case % 2 == 0,
            base_x + off,
            base_y + off2,
            wz,
            &format!("case {case}"),
        );
    }

    let nan = f32::NAN;
    run(&mut emu, (2, 3), false, nan, 100.0, 7.0, "NaN x");
    run(&mut emu, (2, 3), false, 70.0, nan, 7.0, "NaN y");
    run(&mut emu, (2, 3), false, nan, nan, 7.0, "NaN both");
    run(&mut emu, (0, 0), false, -0.0, -0.0, 7.0, "negative zero");
    run(&mut emu, (0, 0), false, 0.0, 0.0, 7.0, "zero");
    run(
        &mut emu,
        (2, 3),
        false,
        f32::INFINITY,
        100.0,
        7.0,
        "infinite x",
    );
    run(
        &mut emu,
        (2, 3),
        false,
        f32::NEG_INFINITY,
        100.0,
        7.0,
        "negative infinite x",
    );
}

/// `0x67ca90` from `0x67cab4` to `0x67cb41`: the world `(x, y)` it passes the adapter for a cell,
/// read from the frame at `[ebp-0x18]` and `[ebp-0x14]`, over tile origins of both signs.
#[test]
fn mist_cell_world_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let (grid, tiles) = alloc_grid(&mut emu);
    let pair = emu.alloc(0x10);
    let stack = emu.alloc(0x1000);
    let mut s = 0x6365_6c6cu32;

    for case in 0..1500 {
        let origin = (
            ((next(&mut s) >> 8) % 400) as i32 - 200,
            ((next(&mut s) >> 8) % 400) as i32 - 200,
        );
        let cellx = ((next(&mut s) >> 8) % 160) as i32;
        let celly = ((next(&mut s) >> 8) % 160) as i32;
        write_grid_origins(&mut emu, grid, origin, (origin.0 << 5, origin.1 << 5));
        let coords = window_coords(origin);
        fill_tiles(&mut emu, &tiles, &coords, &|_| [FLT_MAX_SENTINEL; 0x400]);
        emu.write_u32(pair, cellx as u32);
        emu.write_u32(pair + 4, celly as u32);
        let esp = stack + 0x800;
        emu.write_u32(esp + 4, pair as u32);
        emu.write_u32(esp + 8, 0x40c0_0000);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ECX, grid);
        emu.run(0x67ca90, 0x67cb41);
        let ebp = esp - 4;
        let (wx, wy) = cell_world_xy(cellx, celly, origin);
        assert_f32(
            emu.read_f32(ebp - 0x18),
            wx,
            &format!("x (case {case} cell=({cellx},{celly}) origin={origin:?})"),
        );
        assert_f32(
            emu.read_f32(ebp - 0x14),
            wy,
            &format!("y (case {case} cell=({cellx},{celly}) origin={origin:?})"),
        );
    }
}

/// `0x67cb60` whole, 700 segments: a density crossing at the first point (cells all high), a walk
/// to the end (cells empty), a crossing in the interior (cells random, so the marched point is
/// compared), an endpoint outside the 160 by 160 grid, fractional negative endpoints (floor, not
/// truncation) and the y values where the product stored as f32 floors differently from the wide
/// one, and finite endpoints past the cell range. A NaN or infinite endpoint is not run: `floor`
/// (`0x73ff3f`) faults in the emulator on its error path.
#[test]
fn mist_grid_raymarch_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(GROUND_SAMPLER, 0, 1);

    let p0_p = emu.alloc(0x10);
    let p1_p = emu.alloc(0x10);
    let out_p = emu.alloc(0x10);
    let (grid, tiles) = alloc_grid(&mut emu);
    let mut s = 0x6d61_7263u32;
    let (mut hits, mut ends, mut left, mut interior) = (0u32, 0u32, 0u32, 0u32);

    let cell_origin = (2i32, 4i32);
    let origin_cell = (cell_origin.0 << 5, cell_origin.1 << 5);
    write_grid_origins(&mut emu, grid, cell_origin, origin_cell);
    let tile_coords = window_coords(cell_origin);

    let mut run = |emu: &mut Emu, p0: Vec3, p1: Vec3, class: u32, ctx: &str| {
        let init = |_w: usize| -> [f32; 0x400] { [FLT_MAX_SENTINEL; 0x400] };
        let seed = ctx.bytes().fold(0x1357_9bdfu32, |a, b| {
            a.wrapping_mul(31).wrapping_add(u32::from(b))
        });
        let init_rand = |w: usize| -> [f32; 0x400] {
            let mut a = [0.0f32; 0x400];
            let mut t = seed.wrapping_add(w as u32);
            for v in a.iter_mut() {
                t = t.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *v = (f64::from(t >> 8) / f64::from(1u32 << 24) * 100.0) as f32;
            }
            a
        };
        match class {
            0 => fill_tiles(emu, &tiles, &tile_coords, &|_| [1.0e6f32; 0x400]),
            1 => fill_tiles(emu, &tiles, &tile_coords, &init),
            _ => fill_tiles(emu, &tiles, &tile_coords, &init_rand),
        }
        for (a, p) in [(p0_p, p0), (p1_p, p1)] {
            emu.write_u32(a, p.x.to_bits());
            emu.write_u32(a + 4, p.y.to_bits());
            emu.write_u32(a + 8, p.z.to_bits());
        }
        for k in 0..3 {
            emu.write_u32(out_p + 4 * k, 0);
        }

        let eax = emu.call(
            0x67cb60,
            grid as u32,
            0,
            &[p0_p as u32, p1_p as u32, out_p as u32],
        );
        let bin_hit = (eax & 0xff) != 0;
        let bin_out = Vec3 {
            x: emu.read_f32(out_p),
            y: emu.read_f32(out_p + 4),
            z: emu.read_f32(out_p + 8),
        };

        let mut tiles_ours = read_tiles(emu, &tiles, &tile_coords);
        let terrain = |_x: f32, _y: f32| -1.0e30f32;
        let (our_out, our_hit) =
            mist_grid_raymarch(origin_cell, cell_origin, p0, p1, &mut tiles_ours, terrain);

        assert_eq!(bin_hit, our_hit, "hit flag ({ctx}) bin_eax={eax:#x}");
        assert_f32(bin_out.x, our_out.x, &format!("out.x ({ctx})"));
        assert_f32(bin_out.y, our_out.y, &format!("out.y ({ctx})"));
        assert_f32(bin_out.z, our_out.z, &format!("out.z ({ctx})"));
        if bin_hit {
            hits += 1;
            if bin_out != p0 {
                interior += 1;
            }
        } else if bin_out == p0 {
            left += 1;
        } else {
            ends += 1;
        }
    };

    let mk = |s: &mut u32| -> Vec3 {
        let cx = 10.0 + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 130.0;
        let cy = 10.0 + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 130.0;
        let wx = (origin_cell.0 as f64 + cx) / 0.96f64;
        let wy = (origin_cell.1 as f64 + cy) / 0.96f64;
        let wz = f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 100.0;
        Vec3 {
            x: wx as f32,
            y: wy as f32,
            z: wz as f32,
        }
    };

    for case in 0..700u32 {
        let class = case % 3;
        let mut p0 = mk(&mut s);
        let mut p1 = mk(&mut s);
        if case % 11 == 10 {
            p1.x = 5000.0;
        }
        if case % 13 == 12 {
            p0.x = -123.45 - (next(&mut s) >> 18) as f32 * 0.001;
            p0.y = -67.8 - (next(&mut s) >> 18) as f32 * 0.001;
        }
        run(&mut emu, p0, p1, class, &format!("case {case}"));
    }
    for case in 0..300u32 {
        let mut p0 = mk(&mut s);
        let mut p1 = mk(&mut s);
        p0.z = 80.0 + p0.z * 0.2;
        p1.z = 80.0 + p1.z * 0.2;
        run(&mut emu, p0, p1, 2, &format!("interior {case}"));
    }
    let probes = straddling_y(TILE_SCALE_MARCH);
    assert!(!probes.is_empty(), "no y straddles a cell boundary");
    for (i, &(y, _)) in probes.iter().enumerate().take(40) {
        let mut p0 = mk(&mut s);
        let mut p1 = mk(&mut s);
        p0.y = y;
        p0.x = (origin_cell.0 as f32 + 70.0) / 0.96;
        p1.y = if i % 2 == 0 { y + 60.0 } else { y - 60.0 };
        run(
            &mut emu,
            p0,
            p1,
            2,
            &format!("y rounded before the floor {i}"),
        );
    }
    let base = mk(&mut s);
    run(&mut emu, base, base, 2, "zero length");
    // Finite endpoints whose cell passes `i32` and `i64`: `__ftol` keeps the low dword of its
    // qword, 0 past `i64`, so a segment between two such points is one cell outside the grid.
    for (a, b) in [
        ((1.0e30f32, 1.0e30f32), (-1.0e30f32, 3.0e30f32)),
        ((3.0e38, -3.0e38), (1.0e19, 1.0e25)),
        ((1.0e30, 5.0), (1.0e31, 5.0)),
    ] {
        let p0 = Vec3 {
            x: a.0,
            y: a.1,
            z: 10.0,
        };
        let p1 = Vec3 {
            x: b.0,
            y: b.1,
            z: 20.0,
        };
        run(&mut emu, p0, p1, 2, "endpoints past the cell range");
    }
    assert!(
        hits > 0 && ends > 0 && left > 0 && interior > 50,
        "hit, end, left and interior walks all ran: {hits} {ends} {left} {interior}"
    );
}

/// The generator state `[0x882664]`, `[0x882668]` and its table.
const RNG_ACC: u64 = 0x882664;
const RNG_IDX: u64 = 0x882668;
const RNG_TABLE: u64 = 0x802700;
/// The weather manager pointer.
const MGR: u64 = 0xc6326c;
/// The camera position.
const CAM: u64 = 0xc7cf20;
/// The one-time init flag of the tilt axis.
const TILT_GUARD: u64 = 0xc7b530;
/// The tilt axis.
const TILT_AXIS: u64 = 0xc7b570;
/// The grid table, `+0xc` the height virtual.
const GRID_VTABLE: u64 = 0x810038;
const GROUND_SAMPLER_SPAWN: u64 = 0x6b7070;

/// `0x67a990` whole, 500 cases over tilt angles including 0, camera positions inside the window,
/// and extents, radii and azimuth ranges. The grid is the real one: every cell holds one value, the
/// virtual runs `0x67ca00`, and the transcription's sampler is [`mist_grid_sample`] over the same
/// tiles; the two classes of the z floor (sample above or below the placed z) both run.
#[test]
fn mist_spawn_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(GROUND_SAMPLER_SPAWN, 0, 1);

    let table = emu.read(RNG_TABLE, 256);

    let manager = emu.alloc(0x100);
    emu.write_u32(MGR, manager as u32);
    let grid = emu.alloc(0x100);
    emu.write_u32(manager + 0x34, grid as u32);
    emu.write_u32(grid, GRID_VTABLE as u32);

    let mut tiles = Vec::with_capacity(WINDOW);
    for w in 0..WINDOW {
        let tile = emu.alloc(TILE_BYTES);
        emu.write_u32(grid + 0x28 + 4 * w as u64, tile as u32);
        tiles.push(tile);
    }
    let cell_origin = (3i32, 3i32);
    write_grid_origins(
        &mut emu,
        grid,
        cell_origin,
        (cell_origin.0 << 5, cell_origin.1 << 5),
    );
    let tile_coords = window_coords(cell_origin);

    let src_p = emu.alloc(0x80);
    let out_p = emu.alloc(0x200);
    let mut s = 0x6d73_7077u32;
    let (mut above, mut below) = (0u32, 0u32);

    for case in 0..500 {
        let acc = next(&mut s);
        let idx = next(&mut s);
        emu.write_u32(RNG_ACC, acc);
        emu.write_u32(RNG_IDX, idx);

        emu.write_u32(TILT_AXIS, 0);
        emu.write_u32(TILT_AXIS + 4, 0);
        emu.write_u32(TILT_AXIS + 8, 0x3f80_0000);
        emu.write_u32(TILT_GUARD, 1);
        let tilt_angle = if case % 7 == 0 {
            0.0f32
        } else {
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32
        };
        emu.write_u32(manager + 0x78, tilt_angle.to_bits());

        let cam = [
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 16.0 + 175.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 16.0 + 175.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 20.0 - 10.0) as f32,
        ];
        for (k, &c) in cam.iter().enumerate() {
            emu.write_u32(CAM + 4 * k as u64, c.to_bits());
        }

        let cell_k = if case % 2 == 0 { 50.0f32 } else { -50.0 };
        fill_tiles(&mut emu, &tiles, &tile_coords, &|_| [cell_k; 0x400]);

        let src = MistSpawnSource {
            pos_extent: [
                rand_f(&mut s, 0.5, 3.0),
                rand_f(&mut s, 0.5, 3.0),
                rand_f(&mut s, 0.5, 3.0),
            ],
            cell_floor_scale: rand_f(&mut s, -2.0, 2.0),
            azimuth_base: rand_f(&mut s, -1.0, 1.0),
            azimuth_scale: rand_f(&mut s, 0.0, 6.0),
            radius_base: rand_f(&mut s, 0.5, 2.0),
            radius_scale: rand_f(&mut s, 0.0, 2.0),
        };
        write_src(&mut emu, src_p, &src);
        for k in 0..0x40u64 {
            emu.write_u32(out_p + 0x100 + 4 * k, 0);
        }
        let base_index = next(&mut s);

        emu.call(0x67a990, src_p as u32, 0, &[out_p as u32, base_index]);
        let bin = read_out(&emu, out_p);

        let mut rng = Stream {
            acc,
            idx,
            table: &table,
        };
        let mut tiles_ours = read_tiles(&emu, &tiles, &tile_coords);
        let mut sampled = FLT_MAX_SENTINEL;
        let ours = mist_spawn(
            &src,
            base_index,
            tilt_angle,
            [0.0, 0.0, 1.0],
            cam,
            &mut rng,
            |w| {
                let terrain = |_x: f32, _y: f32| w[2] - HEIGHT_DROP;
                sampled =
                    mist_grid_sample(cell_origin, (w[0], w[1]), &mut tiles_ours, terrain).value;
                sampled
            },
        );
        if f64::from(sampled) < f64::from(ours.world[2]) {
            below += 1;
        } else {
            above += 1;
        }

        let ctx =
            |f: &str| format!("mist_spawn case {case} {f} (tilt={tilt_angle} sample={sampled})");
        assert_f32(bin.dir[0], ours.dir[0], &ctx("dir.x"));
        assert_f32(bin.dir[1], ours.dir[1], &ctx("dir.y"));
        assert_f32(bin.dir[2], ours.dir[2], &ctx("dir.z"));
        assert_f32(bin.world[0], ours.world[0], &ctx("world.x"));
        assert_f32(bin.world[1], ours.world[1], &ctx("world.y"));
        assert_f32(bin.world[2], ours.world[2], &ctx("world.z"));
        assert_f32(bin.tail_scale, ours.tail_scale, &ctx("tail_scale"));
        assert_eq!(bin.base_index, ours.base_index, "{}", ctx("base_index"));
        assert_eq!(bin.tail_index, ours.tail_index, "{}", ctx("tail_index"));
    }
    assert!(
        above > 0 && below > 0,
        "both floor classes ran: {above} {below}"
    );
}

/// A value in `[lo, hi)`.
fn rand_f(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * f64::from(hi - lo)) as f32
}

/// Write the effect fields `0x67a990` reads.
fn write_src(emu: &mut Emu, p: u64, src: &MistSpawnSource) {
    emu.write_u32(p + 0x04, src.pos_extent[0].to_bits());
    emu.write_u32(p + 0x08, src.pos_extent[1].to_bits());
    emu.write_u32(p + 0x0c, src.pos_extent[2].to_bits());
    emu.write_u32(p + 0x1c, src.cell_floor_scale.to_bits());
    emu.write_u32(p + 0x24, src.azimuth_base.to_bits());
    emu.write_u32(p + 0x28, src.azimuth_scale.to_bits());
    emu.write_u32(p + 0x34, src.radius_base.to_bits());
    emu.write_u32(p + 0x38, src.radius_scale.to_bits());
}

/// Read the fields `0x67a990` writes into the puff record.
fn read_out(emu: &Emu, p: u64) -> MistSpawnOut {
    MistSpawnOut {
        world: [
            emu.read_f32(p + 0x108),
            emu.read_f32(p + 0x10c),
            emu.read_f32(p + 0x110),
        ],
        dir: [
            emu.read_f32(p + 0x114),
            emu.read_f32(p + 0x118),
            emu.read_f32(p + 0x11c),
        ],
        base_index: emu.read_u32(p + 0x120),
        tail_index: emu.read_u32(p + 0x124) as i32,
        tail_scale: emu.read_f32(p + 0x128),
    }
}
