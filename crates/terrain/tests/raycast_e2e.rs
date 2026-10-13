//! The traces end to end, each run whole with nothing stubbed: `TraceTerrainSegment` (`0x69c320`)
//! through `0x69c920`, and `TraceTerrainFirstHit` (`0x6aa3e0`) through `0x6aa600`, on a world
//! built along the cells each segment's walk lists.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::dda::{CellIntersector, CellList, Ray6};
use benilla_re_terrain::intersect::{
    FirstIntersector, MapIntersector, RaycastWorld, AREA_TABLE_VA,
};
use benilla_re_terrain::{ray_dda_chunk, ray_dda_map};
use common::{assert_f32, next};

const CELL_PTR: u64 = 0x00c9_62c8;

const CELL_CNT: u64 = 0x00c8_9f40;

const OFF_HEIGHTS: u64 = 0x83c;

const OFF_HOLEPTR: u64 = 0xf08;

const OFF_HOLEMASK: u64 = 0x3c;

const OFF_LIQ: u64 = 0x118;

const OFF_LIQ_TYPE: u64 = 0x10;

const OFF_LIQ_VERTS: u64 = 0x34;

/// The world in the emulator's memory, read as in `tests/intersect.rs`.
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

/// A miss, to take the cell list from the reimplementation's walk.
struct MissIntersector;

impl CellIntersector for MissIntersector {
    fn intersect(&mut self, _list: &CellList, _ray: &Ray6, _t_io: &mut f32) -> bool {
        false
    }
}

/// Builds the tiles and chunks the cells resolve to as they are needed: each chunk a sheet of
/// heights about `(cx, cy)` with z in `[zbase, zbase + 3]`, a hole block without holes unless
/// `holed`, and a flat liquid at `liq_z` with `with_liquid`.
struct Builder {
    tiles: std::collections::HashMap<usize, u64>,
    chunks: std::collections::HashMap<(usize, usize), u64>,
}

impl Builder {
    fn new(emu: &mut Emu) -> Builder {
        for i in 0..0x1000u64 {
            emu.write_u32(u64::from(AREA_TABLE_VA) + i * 4, 0);
        }
        Builder {
            tiles: std::collections::HashMap::new(),
            chunks: std::collections::HashMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ensure_cells(
        &mut self,
        emu: &mut Emu,
        cells: &[(i32, i32)],
        cx: f32,
        cy: f32,
        zbase: f32,
        holed: bool,
        with_liquid: bool,
        liq_z: f32,
        s: &mut u32,
    ) {
        for &(c0_i, c1_i) in cells {
            let (c0, c1) = (c0_i as u32, c1_i as u32);
            if c0 > 0x2000 || c1 > 0x2000 {
                continue;
            }
            let grid_index = (((c1 >> 7) & 0x3f) * 0x40 + ((c0 >> 7) & 0x3f)) as usize;
            let tile = *self.tiles.entry(grid_index).or_insert_with(|| {
                let t = emu.alloc(0x278 + 256 * 4 + 0x10);
                emu.write_u32(u64::from(AREA_TABLE_VA) + grid_index as u64 * 4, t as u32);
                t
            });
            let chunk_index = (((c1 >> 3) & 0xf) * 0x10 + ((c0 >> 3) & 0xf)) as usize;
            let key = (grid_index, chunk_index);
            if self.chunks.contains_key(&key) {
                continue;
            }
            let chunk = emu.alloc(0x2000);
            emu.write_u32(tile + 0x278 + chunk_index as u64 * 4, chunk as u32);
            // 17×17 heights over the segment's x and y, a gentle slope below the ray
            for row in 0..17i64 {
                for col in 0..17i64 {
                    let idx = (row * 17 + col) as u64;
                    let base = chunk + OFF_HEIGHTS + idx * 0xc;
                    let jx = (next(s) >> 8) as f64 / (1u32 << 24) as f64;
                    let jz = (next(s) >> 8) as f64 / (1u32 << 24) as f64;
                    let x = f64::from(cx) + (col as f64 - 8.0) * 4.5 + jx * 0.5;
                    let y = f64::from(cy) + (row as f64 - 8.0) * 4.5 + jx * 0.3;
                    let z = f64::from(zbase) + ((col + row) as f64) * 0.08 + jz * 2.0;
                    emu.write_u32(base, (x as f32).to_bits());
                    emu.write_u32(base + 4, (y as f32).to_bits());
                    emu.write_u32(base + 8, (z as f32).to_bits());
                }
            }
            let hole_block = emu.alloc(0x40);
            emu.write_u32(hole_block + OFF_HOLEMASK, if holed { 0xffff } else { 0 });
            emu.write_u32(chunk + OFF_HOLEPTR, hole_block as u32);
            if with_liquid {
                let layer = emu.alloc(0x400);
                let types = emu.alloc(0x40);
                for b in (0..0x40u64).step_by(4) {
                    emu.write_u32(types + b, 0); // nibble 0, wet
                }
                emu.write_u32(layer + OFF_LIQ_TYPE, types as u32);
                for row in 0..9i64 {
                    for col in 0..9i64 {
                        let idx = (row * 9 + col) as u64;
                        let base = layer + OFF_LIQ_VERTS + idx * 0xc;
                        let x = f64::from(cx) + (col as f64 - 4.0) * 9.0;
                        let y = f64::from(cy) + (row as f64 - 4.0) * 9.0;
                        emu.write_u32(base, (x as f32).to_bits());
                        emu.write_u32(base + 4, (y as f32).to_bits());
                        emu.write_u32(base + 8, liq_z.to_bits());
                    }
                }
                emu.write_u32(chunk + OFF_LIQ, layer as u32);
            } else {
                emu.write_u32(chunk + OFF_LIQ, 0);
            }
            emu.write_u32(chunk + OFF_LIQ + 4, 0);
            emu.write_u32(chunk + OFF_LIQ + 8, 0);
            emu.write_u32(chunk + OFF_LIQ + 0xc, 0);
            self.chunks.insert(key, chunk);
        }
    }
}

/// The reimplementation's cell list for a segment, its cell test a miss.
fn walk_cells(a: &[f32; 3], b: &[f32; 3]) -> CellList {
    let mut list = CellList::default();
    let mut t = 0.0f32;
    ray_dda_map(a, b, &mut t, &mut list, &mut MissIntersector);
    list
}

#[test]
fn raycast_e2e_map_driver() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pa = emu.alloc(0x10);
    let pb = emu.alloc(0x10);
    let p_t = emu.alloc(8);
    let p_chunk = emu.alloc(8);
    let mut s: u32 = 0xe2e0_0001;
    let mut builder = Builder::new(&mut emu);
    let buf = emu.alloc(0x40000);
    emu.write_u32(CELL_PTR, buf as u32);

    // Eight fixed places, so chunks are shared between cases (the arena cannot hold a chunk a
    // case); place 0's chunks are all holes; every chunk has a liquid, which the mask turns on or
    // off.
    let anchors: [(f32, f32); 8] = [
        (15000.0, 15000.0),
        (15200.0, 15100.0),
        (15400.0, 15350.0),
        (15650.0, 15600.0),
        (16000.0, 15800.0),
        (16200.0, 16100.0),
        (16500.0, 16400.0),
        (16800.0, 16700.0),
    ];
    let (mut hits, mut misses, mut liq_hits, mut holed_cases) = (0u32, 0u32, 0u32, 0u32);
    for case in 0..1500usize {
        let ai = ((next(&mut s) >> 8) as usize) % anchors.len();
        let (ax, ay) = anchors[ai];
        let holed = ai == 0;
        // a short segment, mostly downward, about the place
        let wx = ax + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 40.0 - 20.0;
        let wy = ay + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 40.0 - 20.0;
        let a = [wx, wy, 60.0];
        let b = [
            wx + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 12.0 - 6.0,
            wy + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 12.0 - 6.0,
            -40.0,
        ];
        let with_liquid = (next(&mut s) >> 8).is_multiple_of(2);
        let mask = if with_liquid { 0x100 | 0x10000 } else { 0x100 };
        // the world along the reimplementation's walk, the heights about the place, the liquid at
        // z = 20
        let cells = walk_cells(&a, &b);
        builder.ensure_cells(
            &mut emu,
            &cells.pairs(),
            ax,
            ay,
            5.0,
            holed,
            true,
            20.0,
            &mut s,
        );
        if holed {
            holed_cases += 1;
        }

        let t_in = 1.0e30f32;
        for (i, &c) in a.iter().enumerate() {
            emu.write_u32(pa + i as u64 * 4, c.to_bits());
        }
        for (i, &c) in b.iter().enumerate() {
            emu.write_u32(pb + i as u64 * 4, c.to_bits());
        }
        emu.write_u32(p_t, t_in.to_bits());
        emu.write_u32(p_chunk, 0);
        emu.write_u32(CELL_CNT, 0xdead_beef); // the trace must zero it

        // ecx = A, edx = B, &t, mask and &out_chunk on the stack
        emu.call(
            0x69c320,
            pa as u32,
            pb as u32,
            &[p_t as u32, mask, p_chunk as u32],
        );
        let bin_t = emu.read_f32(p_t);
        let bin_chunk = emu.read_u32(p_chunk);
        let bin_cells: Vec<i32> = {
            let base = emu.read_u32(CELL_PTR) as u64;
            let n = emu.read_u32(CELL_CNT) as usize;
            (0..n)
                .map(|i| emu.read_u32(base + i as u64 * 4) as i32)
                .collect()
        };

        // the reimplementation's walk and cell test
        let world = EmuWorld { emu: &emu };
        let mut isect = MapIntersector {
            world: &world,
            mask,
            out_chunk: 0,
            hit: false,
        };
        let mut list = CellList::default();
        let mut t_ours = t_in;
        ray_dda_map(&a, &b, &mut t_ours, &mut list, &mut isect);

        assert_eq!(bin_cells, list.data, "case {case}: visited cells");
        assert_f32(
            bin_t,
            t_ours,
            &format!("case {case}: t_io (mask={mask:#x})"),
        );
        if isect.hit {
            assert_eq!(bin_chunk, isect.out_chunk, "case {case}: hit chunk");
            hits += 1;
            if with_liquid {
                liq_hits += 1;
            }
        } else {
            assert_f32(bin_t, t_in, &format!("case {case}: miss t unchanged"));
            misses += 1;
        }
    }
    eprintln!(
        "raycast_e2e_map_driver: 1500 segments bit-exact vs UNSTUBBED 0x69c320 \
         (hits={hits} misses={misses} liquid_enabled_hits={liq_hits} holed_cases={holed_cases})"
    );
    assert!(hits > 0, "no end-to-end hit fired");
    assert!(misses > 0, "no end-to-end miss fired");
    assert!(liq_hits > 0, "no liquid-enabled hit fired");
    assert!(holed_cases > 0, "no holed-chunk case fired");
}

#[test]
fn raycast_e2e_chunk_driver() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p_seg = emu.alloc(0x20);
    let p_t = emu.alloc(8);
    let p_plane = emu.alloc(0x20);
    let p_dummy = emu.alloc(0x20);
    let mut s: u32 = 0xe2e0_c000;
    let mut builder = Builder::new(&mut emu);
    let buf = emu.alloc(0x40000);
    emu.write_u32(CELL_PTR, buf as u32);

    let anchors: [(f32, f32); 8] = [
        (15000.0, 15000.0),
        (15200.0, 15100.0),
        (15400.0, 15350.0),
        (15650.0, 15600.0),
        (16000.0, 15800.0),
        (16200.0, 16100.0),
        (16500.0, 16400.0),
        (16800.0, 16700.0),
    ];
    let (mut hits, mut misses, mut scaled) = (0u32, 0u32, 0u32);
    for case in 0..1500usize {
        let ai = ((next(&mut s) >> 8) as usize) % anchors.len();
        let (ax, ay) = anchors[ai];
        let wx = ax + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 40.0 - 20.0;
        let wy = ay + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 40.0 - 20.0;
        let a = [wx, wy, 60.0];
        // As `0x6aa160` casts it: B − A a direction of about one unit and t the range, so the walk
        // covers A to B while the cell test's ray reaches A + (B − A)·t0. A hit needs `t < t0`, so
        // t0 must reach the surface, about 55 units below A.
        let b = [
            wx + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 0.2 - 0.1,
            wy + ((next(&mut s) >> 8) as f32 / (1u32 << 24) as f32) * 0.2 - 0.1,
            59.0, // a downward direction (jx, jy, −1)
        ];
        // t0 is 100, and 75 every few cases, so the rescale `t·t0` is seen with two scales
        let t0 = if (next(&mut s) >> 8).is_multiple_of(3) {
            scaled += 1;
            75.0f32
        } else {
            100.0f32
        };
        let seg = [a[0], a[1], a[2], b[0], b[1], b[2]];
        // the walk covers A to B; the test's ray reaches t0 along it, in the same cells
        let cells = walk_cells(&a, &b);
        builder.ensure_cells(
            &mut emu,
            &cells.pairs(),
            ax,
            ay,
            5.0,
            false,
            false,
            0.0,
            &mut s,
        );

        for (i, &c) in seg.iter().enumerate() {
            emu.write_u32(p_seg + i as u64 * 4, c.to_bits());
        }
        emu.write_u32(p_t, t0.to_bits());
        for i in 0..4u64 {
            emu.write_u32(p_plane + i * 4, 0xdead_beef);
        }
        emu.write_u32(CELL_CNT, 0xdead_beef);

        // ecx = seg, edx = &t, 2 arguments on the stack
        let ret = emu.call(
            0x6aa3e0,
            p_seg as u32,
            p_t as u32,
            &[p_plane as u32, p_dummy as u32],
        );
        let bin_t = emu.read_f32(p_t);
        let bin_plane = [
            emu.read_f32(p_plane),
            emu.read_f32(p_plane + 4),
            emu.read_f32(p_plane + 8),
            emu.read_f32(p_plane + 0xc),
        ];

        let world = EmuWorld { emu: &emu };
        let mut isect = FirstIntersector {
            world: &world,
            plane: None,
        };
        let mut list = CellList::default();
        let mut t_ours = t0;
        let hit = ray_dda_chunk(&seg, &mut t_ours, &mut list, &mut isect);

        assert_eq!(
            ret & 0xff,
            u32::from(hit),
            "case {case}: hit flag binary={} ours={hit}",
            ret & 0xff
        );
        assert_f32(bin_t, t_ours, &format!("case {case}: t_io (t0={t0})"));
        if hit {
            let plane = isect.plane.expect("hit must record a plane");
            for i in 0..4 {
                assert_f32(bin_plane[i], plane[i], &format!("case {case}: plane[{i}]"));
            }
            hits += 1;
        } else {
            assert_f32(bin_t, t0, &format!("case {case}: miss t unchanged"));
            misses += 1;
        }
    }
    eprintln!(
        "raycast_e2e_chunk_driver: 1500 segments bit-exact vs UNSTUBBED 0x6aa3e0 \
         (hits={hits} misses={misses} t0_scaled_cases={scaled})"
    );
    assert!(hits > 0, "no end-to-end first-hit fired");
    assert!(misses > 0, "no end-to-end miss fired");
    assert!(scaled > 0, "no t0-rescale case fired");
}
