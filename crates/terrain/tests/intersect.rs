//! The cell tests, each run whole on a world the test lays out: `TraceListedCells` (`0x69c920`)
//! with the terrain, and the terrain and liquid layer 0, in its mask, through
//! `IntersectRayTriangle` (`0x7c2c40`); and `TraceListedCellsFirstHit` (`0x6aa600`) through
//! `0x7c2c40` and `PlaneFromTriangle` (`0x637480`).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::dda::CellList;
use benilla_re_terrain::intersect::{
    dda_cell_intersect, dda_cell_intersect_first, RaycastWorld, AREA_TABLE_VA,
};
use common::{assert_f32, next};

// The cell list's globals and the chunk's offsets.
const CELL_PTR: u64 = 0x00c9_62c8; // the list's buffer

const CELL_CNT: u64 = 0x00c8_9f40; // the count of integers

const OFF_HEIGHTS: u64 = 0x83c; // the 145 vertices

const OFF_HOLEPTR: u64 = 0xf08; // the pointer to the hole block

const OFF_HOLEMASK: u64 = 0x3c; // the hole bits in that block

const OFF_LIQ: u64 = 0x118; // four liquid pointers

const OFF_LIQ_TYPE: u64 = 0x10; // the liquid's type bytes, 8 a row

const OFF_LIQ_VERTS: u64 = 0x34; // the liquid's 81 vertices

/// A finite `f32` in `[-100, 100]`, so the world is small and the rays hit; NaN is left out.
fn rf(s: &mut u32) -> f32 {
    let u = (next(s) >> 8) as f64 / (1u32 << 24) as f64;
    (u * 200.0 - 100.0) as f32
}

struct EmuWorld<'a, 'b> {
    emu: &'a Emu<'b>,
}

impl RaycastWorld for EmuWorld<'_, '_> {
    fn tile_ptr(&self, grid_index: usize) -> u32 {
        self.emu
            .read_u32(u64::from(AREA_TABLE_VA) + grid_index as u64 * 4)
    }
    fn chunk_ptr(&self, tile: u32, chunk_index: usize) -> u32 {
        self.emu
            .read_u32(u64::from(tile) + 0x278 + chunk_index as u64 * 4)
    }
    fn hole_mask(&self, chunk: u32) -> u16 {
        let block = self.emu.read_u32(u64::from(chunk) + OFF_HOLEPTR);
        (self.emu.read_u32(u64::from(block) + OFF_HOLEMASK) & 0xffff) as u16
    }
    fn terrain_vert(&self, chunk: u32, index: i32) -> [f32; 3] {
        let base = u64::from(chunk) + OFF_HEIGHTS + (index as i64 as u64).wrapping_mul(0xc);
        [
            self.emu.read_f32(base),
            self.emu.read_f32(base + 4),
            self.emu.read_f32(base + 8),
        ]
    }
    fn liquid_layer(&self, chunk: u32, layer: usize) -> u32 {
        self.emu
            .read_u32(u64::from(chunk) + OFF_LIQ + layer as u64 * 4)
    }
    fn liquid_type_nibble(&self, layer: u32, x: i32, y: i32) -> u8 {
        let types = self.emu.read_u32(u64::from(layer) + OFF_LIQ_TYPE);
        let addr = u64::from(types) + (y as i64 as u64).wrapping_mul(8) + (x as i64 as u64);
        // a byte from the dword that holds it; the function loads a byte
        let dword = self.emu.read_u32(addr & !3);
        let shift = (addr & 3) * 8;
        (((dword >> shift) & 0xff) as u8) & 0xf
    }
    fn liquid_vert(&self, layer: u32, index: i32) -> [f32; 3] {
        let base = u64::from(layer) + OFF_LIQ_VERTS + (index as i64 as u64).wrapping_mul(0xc);
        [
            self.emu.read_f32(base),
            self.emu.read_f32(base + 4),
            self.emu.read_f32(base + 8),
        ]
    }
}

struct World {
    /// Cells `(c0, c1)` that resolve to a chunk's cell that is not a hole.
    good_cells: Vec<(u32, u32)>,
    /// A cell in a hole.
    hole_cell: (u32, u32),
    /// A cell whose tile is absent, which ends the walk.
    absent_cell: (u32, u32),
}

/// Lays out the world: heights on a gentle slope so the downward rays hit, and the hole mask on one
/// chunk's cell (0, 0).
fn build_world(emu: &mut Emu, s: &mut u32) -> World {
    // clear the 64×64 tile pointers
    for i in 0..0x1000u64 {
        emu.write_u32(u64::from(AREA_TABLE_VA) + i * 4, 0);
    }
    // a tile, a hole block, a few chunks and their liquids
    let tile = emu.alloc(0x278 + 256 * 4 + 0x10);
    let n_chunks = 6usize;
    let mut chunks = Vec::new();
    for _ in 0..n_chunks {
        // the vertices at +0x83c take 0xd8c bytes; 0x2000 keeps them out of the next chunk's
        // +0x118 and +0xf08
        let chunk = emu.alloc(0x2000);
        // the vertices: (column, row) for x and y, z a gentle slope
        for row in 0..17i64 {
            for col in 0..17i64 {
                let idx = (row * 17 + col) as u64;
                let base = chunk + OFF_HEIGHTS + idx * 0xc;
                let x = (col as f64 - 8.0) * 3.0 + (next(s) >> 8) as f64 / (1u32 << 24) as f64;
                let y = (row as f64 - 8.0) * 3.0 + (next(s) >> 8) as f64 / (1u32 << 24) as f64;
                let z = ((col + row) as f64) * 0.1 + rf(s) as f64 * 0.01;
                emu.write_u32(base, (x as f32).to_bits());
                emu.write_u32(base + 4, (y as f32).to_bits());
                emu.write_u32(base + 8, (z as f32).to_bits());
            }
        }
        // the hole block, its mask 0, at +0xf08
        let hole_block = emu.alloc(0x40);
        emu.write_u32(hole_block + OFF_HOLEMASK, 0);
        emu.write_u32(chunk + OFF_HOLEPTR, hole_block as u32);
        // liquid layer 0: flat water, every type nibble 0, so it is wet
        let layer = emu.alloc(0x400);
        let types = emu.alloc(0x40); // 8 bytes a row, 8 rows
        for b in 0..0x40u64 {
            emu.write_u32(types + b, 0); // nibble 0
        }
        emu.write_u32(layer + OFF_LIQ_TYPE, types as u32);
        for row in 0..9i64 {
            for col in 0..9i64 {
                let idx = (row * 9 + col) as u64;
                let base = layer + OFF_LIQ_VERTS + idx * 0xc;
                let x = (col as f64 - 4.0) * 6.0;
                let y = (row as f64 - 4.0) * 6.0;
                let z = 2.0 + rf(s) as f64 * 0.005;
                emu.write_u32(base, (x as f32).to_bits());
                emu.write_u32(base + 4, (y as f32).to_bits());
                emu.write_u32(base + 8, (z as f32).to_bits());
            }
        }
        emu.write_u32(chunk + OFF_LIQ, layer as u32); // layer 0
        emu.write_u32(chunk + OFF_LIQ + 4, 0); // layers 1 to 3 absent
        emu.write_u32(chunk + OFF_LIQ + 8, 0);
        emu.write_u32(chunk + OFF_LIQ + 0xc, 0);
        chunks.push(chunk);
    }

    // The cells as the resolve reads them, c1 the major index:
    //   c1 = (t1 << 7) | (k1 << 3) | q1, c0 = (t0 << 7) | (k0 << 3) | q0,
    //   tile = t1·64 + t0, chunk = k1·16 + k0.
    // One tile, each chunk its own (k1, k0).
    let t1 = 3u32; // c1's tile, the major index
    let t0 = 5u32; // c0's tile
    emu.write_u32(
        u64::from(AREA_TABLE_VA) + (t1 as u64 * 0x40 + t0 as u64) * 4,
        tile as u32,
    );
    let mut good_cells = Vec::new();
    // each chunk's (k1, k0)
    let chunk_slots = [(2u32, 2u32), (3, 2), (2, 3), (4, 5), (6, 1), (1, 6)];
    for (ci, chunk) in chunks.iter().enumerate() {
        let (k1, k0) = chunk_slots[ci];
        let chunk_index = (k1 * 0x10 + k0) as u64;
        emu.write_u32(tile + 0x278 + chunk_index * 4, *chunk as u32);
        // a few cells per chunk, not in the hole
        for &(q1, q0) in &[(0u32, 0u32), (3, 5), (7, 7), (2, 6), (5, 1)] {
            let c1 = (t1 << 7) | (k1 << 3) | q1;
            let c0 = (t0 << 7) | (k0 << 3) | q0;
            good_cells.push((c0, c1)); // (c0, c1)
        }
    }

    // a cell in a hole: chunk 0's cell (0, 0), table index 0, bit 0
    let (k1_0, k0_0) = chunk_slots[0];
    let hole_block0 = emu.read_u32(chunks[0] + OFF_HOLEPTR);
    emu.write_u32(u64::from(hole_block0) + OFF_HOLEMASK, 0x1);
    let hole_cell = (
        (t0 << 7) | (k0_0 << 3), // c0, q0 = 0
        (t1 << 7) | (k1_0 << 3), // c1, q1 = 0
    );

    // a cell whose tile is absent
    let absent_cell = ((10u32 << 7), (20u32 << 7)); // (c0, c1)

    World {
        good_cells,
        hole_cell,
        absent_cell,
    }
}

/// Writes the cells `(c0, c1)` to the buffer, c0 first, and sets the count.
fn write_cell_list(emu: &mut Emu, buf: u64, cells: &[(u32, u32)]) {
    for (i, &(c0, c1)) in cells.iter().enumerate() {
        emu.write_u32(buf + (i as u64 * 2) * 4, c0);
        emu.write_u32(buf + (i as u64 * 2 + 1) * 4, c1);
    }
    emu.write_u32(CELL_CNT, (cells.len() * 2) as u32);
}

/// The same cells as a `CellList`.
fn rust_cell_list(cells: &[(u32, u32)]) -> CellList {
    let mut list = CellList::default();
    for &(c0, c1) in cells {
        list.data.push(c0 as i32);
        list.data.push(c1 as i32);
    }
    list
}

/// `0x69c920` over lists of the world's cells, some after a hole cell and some ending at the
/// absent one, with rays downward and random. Each case is counted and must occur: the cached
/// chunk, a fresh one, the absent tile, the hole, a hit and a miss of the triangles, the liquid on
/// and off, an improved fraction and not.
#[test]
fn dda_cell_intersect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // the inputs first, the cell buffer last and large
    let pa = emu.alloc(0x10);
    let pb = emu.alloc(0x10);
    let p_t = emu.alloc(8);
    let p_chunk = emu.alloc(8);
    let mut s: u32 = 0x69c9_2000;
    let world = build_world(&mut emu, &mut s);
    let buf = emu.alloc(0x40000);
    emu.write_u32(CELL_PTR, buf as u32);

    let (mut hit_cnt, mut miss_cnt, mut hole_seen, mut absent_seen, mut liquid_hit) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut checked = 0u32;

    for case in 0..4000usize {
        // good cells, sometimes after the hole cell, sometimes ending at the absent one
        let mut cells: Vec<(u32, u32)> = Vec::new();
        let r = next(&mut s);
        if (r >> 8).is_multiple_of(5) {
            cells.push(world.hole_cell);
            hole_seen += 1;
        }
        let n = 1 + ((next(&mut s) >> 8) as usize % 6);
        for _ in 0..n {
            let gi = (next(&mut s) >> 8) as usize % world.good_cells.len();
            cells.push(world.good_cells[gi]);
        }
        if (next(&mut s) >> 8).is_multiple_of(7) {
            cells.push(world.absent_cell);
            absent_seen += 1;
        }

        // from above the surface, down and jittered, so the triangles are hit often
        let a = [rf(&mut s) * 0.3, rf(&mut s) * 0.3, 40.0 + rf(&mut s) * 0.1];
        let b = [
            a[0] + rf(&mut s) * 0.2,
            a[1] + rf(&mut s) * 0.2,
            a[2] - 80.0,
        ];
        let t_in = 1.0e30f32; // a large t, so any hit improves it

        // the terrain alone, or with liquid layer 0
        let with_liquid = (next(&mut s) >> 8).is_multiple_of(2);
        let mask = if with_liquid { 0x100 | 0x10000 } else { 0x100 };

        // the exe
        for (i, &c) in [a[0], a[1], a[2]].iter().enumerate() {
            emu.write_u32(pa + i as u64 * 4, c.to_bits());
        }
        for (i, &c) in [b[0], b[1], b[2]].iter().enumerate() {
            emu.write_u32(pb + i as u64 * 4, c.to_bits());
        }
        emu.write_u32(p_t, t_in.to_bits());
        emu.write_u32(p_chunk, 0xdead_beef);
        write_cell_list(&mut emu, buf, &cells);
        // ecx = A, edx = B, &t, mask and &out_chunk on the stack, `ret 0xc`
        let ret = emu.call(
            0x69c920,
            pa as u32,
            pb as u32,
            &[p_t as u32, mask, p_chunk as u32],
        );
        let bin_t = emu.read_f32(p_t);
        let bin_chunk = emu.read_u32(p_chunk);

        // the reimplementation
        let list = rust_cell_list(&cells);
        let world_view = EmuWorld { emu: &emu };
        let ours = dda_cell_intersect(&world_view, &list, &a, &b, t_in, mask);

        assert_eq!(
            ret & 0xff,
            u32::from(ours.hit),
            "case {case}: hit flag binary={} ours={}",
            ret & 0xff,
            ours.hit
        );
        assert_f32(bin_t, ours.t, &format!("case {case}: t (mask={mask:#x})"));
        if ours.hit {
            assert_eq!(
                bin_chunk, ours.chunk,
                "case {case}: hit chunk binary={bin_chunk:#x} ours={:#x}",
                ours.chunk
            );
            hit_cnt += 1;
            if with_liquid && ours.t > 0.0 {
                // with the liquid on, any hit counts toward the liquid case
                liquid_hit += 1;
            }
        } else {
            // a miss leaves the outputs
            assert_f32(bin_t, t_in, &format!("case {case}: miss t unchanged"));
            miss_cnt += 1;
        }
        checked += 1;
    }

    eprintln!(
        "dda_cell_intersect: {checked} cases bit-exact vs 0x69c920 \
         (hit={hit_cnt} miss={miss_cnt} hole_prefixed={hole_seen} absent_suffixed={absent_seen} liquid_enabled_hits={liquid_hit})"
    );
    assert!(hit_cnt > 0, "no terrain hit fired");
    assert!(miss_cnt > 0, "no all-miss fired");
    assert!(hole_seen > 0, "hole-reject branch never exercised");
    assert!(absent_seen > 0, "tile-absent stop never exercised");
    assert!(liquid_hit > 0, "liquid mask path never produced a hit");
}

/// `0x6aa600` over the same lists, its plane compared: segments long enough and shorter than
/// 0.01, the cached and fresh chunk and the absent tile, a hole skipped, the first hit stopping
/// the walk, misses to the next cell and to the end, and hits below and above the fraction.
#[test]
fn dda_cell_intersect_first_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p_seg = emu.alloc(0x20); // A at 0..3, B at 3..6
    let p_t = emu.alloc(8);
    let p_plane = emu.alloc(0x20); // the plane 0x637480 writes
    let mut s: u32 = 0x6aa6_0000;
    let world = build_world(&mut emu, &mut s);
    let buf = emu.alloc(0x40000);
    emu.write_u32(CELL_PTR, buf as u32);

    let (mut hit_cnt, mut miss_cnt, mut hole_seen, mut absent_seen, mut gate_rejected) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut checked = 0u32;

    let run = |emu: &mut Emu, seg: &[f32; 6], t_in: f32, cells: &[(u32, u32)]| {
        for (i, &c) in seg.iter().enumerate() {
            emu.write_u32(p_seg + i as u64 * 4, c.to_bits());
        }
        emu.write_u32(p_t, t_in.to_bits());
        for i in 0..4u64 {
            emu.write_u32(p_plane + i * 4, 0xdead_beef);
        }
        write_cell_list(emu, buf, cells);
        // ecx = seg, edx = &t, &plane on the stack, `ret 8`
        let ret = emu.call(0x6aa600, p_seg as u32, p_t as u32, &[p_plane as u32]);
        let bin_t = emu.read_f32(p_t);
        let bin_plane = [
            emu.read_f32(p_plane),
            emu.read_f32(p_plane + 4),
            emu.read_f32(p_plane + 8),
            emu.read_f32(p_plane + 0xc),
        ];
        (ret & 0xff, bin_t, bin_plane)
    };

    for case in 0..4000usize {
        let mut cells: Vec<(u32, u32)> = Vec::new();
        if (next(&mut s) >> 8).is_multiple_of(5) {
            cells.push(world.hole_cell);
            hole_seen += 1;
        }
        let n = 1 + ((next(&mut s) >> 8) as usize % 6);
        for _ in 0..n {
            let gi = (next(&mut s) >> 8) as usize % world.good_cells.len();
            cells.push(world.good_cells[gi]);
        }
        if (next(&mut s) >> 8).is_multiple_of(7) {
            cells.push(world.absent_cell);
            absent_seen += 1;
        }

        let a = [rf(&mut s) * 0.3, rf(&mut s) * 0.3, 40.0 + rf(&mut s) * 0.1];
        let b = [
            a[0] + rf(&mut s) * 0.2,
            a[1] + rf(&mut s) * 0.2,
            a[2] - 80.0,
        ];
        let seg = [a[0], a[1], a[2], b[0], b[1], b[2]];
        let t_in = 1.0e30f32;

        let (bin_flag, bin_t, bin_plane) = run(&mut emu, &seg, t_in, &cells);

        let list = rust_cell_list(&cells);
        let world_view = EmuWorld { emu: &emu };
        let ours = dda_cell_intersect_first(&world_view, &list, &seg, t_in);

        assert_eq!(
            bin_flag,
            u32::from(ours.hit),
            "case {case}: first-hit flag binary={bin_flag} ours={}",
            ours.hit
        );
        assert_f32(bin_t, ours.t, &format!("case {case}: first-hit t"));
        if ours.hit {
            let plane = ours.plane.expect("hit must record a plane");
            for i in 0..4 {
                assert_f32(bin_plane[i], plane[i], &format!("case {case}: plane[{i}]"));
            }
            hit_cnt += 1;
        } else {
            assert_f32(bin_t, t_in, &format!("case {case}: miss t unchanged"));
            miss_cnt += 1;
        }
        checked += 1;
    }

    // A segment of length 0: below 0.01, so it returns at once with t and the plane untouched.
    {
        let cells = vec![world.good_cells[0]];
        let a = [1.0f32, 2.0, 30.0];
        let seg = [a[0], a[1], a[2], a[0], a[1], a[2]]; // B = A
        let t_in = 1.0e30f32;
        let (bin_flag, bin_t, _bin_plane) = run(&mut emu, &seg, t_in, &cells);
        let list = rust_cell_list(&cells);
        let world_view = EmuWorld { emu: &emu };
        let ours = dda_cell_intersect_first(&world_view, &list, &seg, t_in);
        assert_eq!(bin_flag, 0, "min-len gate: binary must return 0");
        assert!(!ours.hit, "min-len gate: ours must return 0");
        assert_f32(bin_t, ours.t, "min-len gate: t unchanged");
        gate_rejected += 1;
    }

    eprintln!(
        "dda_cell_intersect_first: {checked} cases + 1 gate-probe bit-exact vs 0x6aa600 \
         (hit={hit_cnt} miss={miss_cnt} hole_prefixed={hole_seen} absent_suffixed={absent_seen} gate_rejected={gate_rejected})"
    );
    assert!(hit_cnt > 0, "no first-hit fired");
    assert!(miss_cnt > 0, "no all-miss fired");
    assert!(hole_seen > 0, "hole-reject branch never exercised");
    assert!(absent_seen > 0, "tile-absent stop never exercised");
    assert!(gate_rejected > 0, "min-length gate never exercised");
}
