//! The light window and the portal planes, as slices: `LinkLightToChunks` (`0x6c0f50`) from
//! `0x6c0f56` to `0x6c0ff0`, and `InitSceneTables` (`0x680610`) from its entry to its `ret` at
//! `0x6807e9`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::geomscale::{register_light_window, scene_cull_planes};
use common::{assert_f32, next};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

const CULL_SLOTS: [u64; 12] = [
    0xc7d0d8, 0xc7d0dc, 0xc7d0e0, // plane 0's x, y, z
    0xc7d0e8, 0xc7d0ec, 0xc7d0f0, // plane 1
    0xc7d0f8, 0xc7d0fc, 0xc7d100, // plane 2
    0xc7d108, 0xc7d10c, 0xc7d110, // plane 3
];

/// `0x6c0f56` to `0x6c0ff0`, before `mov ebx,[ebp-0xc]`: ecx a light whose box is at +0x44 to
/// +0x54; the four indices are read from `[ebp-8]`, `[ebp-0x10]`, `[ebp-0x18]` and `[ebp-0xc]`.
#[test]
fn register_light_window_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let aabb = e.alloc(0x80); // ecx: the box at +0x44 to +0x54
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
        e.run(0x6c0f56, 0x6c0ff0);
        let bin = [
            e.read_u32(ebp - 0x8) as i32,  // max x
            e.read_u32(ebp - 0x10) as i32, // min x
            e.read_u32(ebp - 0x18) as i32, // max y
            e.read_u32(ebp - 0xc) as i32,  // min y
        ];
        let ours = register_light_window(max_x, min_x, max_y, min_y);
        assert_eq!(bin, ours, "{ctx}: binary={bin:?} ours={ours:?}");
    };

    // Edges: coordinates at 17066.666 (distance 0), ties of `fistp` on the chunk edges, and all
    // four equal.
    let gmax = f32::from_bits(0x4685_5555); // 17066.666
    run_case(
        &mut e,
        gmax,
        gmax,
        gmax,
        gmax,
        "rlw at-grid-max (tile_rel 0)",
    );
    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, "rlw all-zero");
    // a coordinate whose `0.03·r − 0.5` is a half integer
    run_case(&mut e, 16000.0, 15000.0, 16000.0, 15000.0, "rlw mid-tile");
    run_case(&mut e, 17066.0, 0.0, 8533.0, -100.0, "rlw spread");

    // ties: `0.03·r` near k + 0.5
    for k in 0..64i32 {
        let tile_rel = (k as f32) / 0.03 + 0.5 / 0.03;
        let coord = gmax - tile_rel;
        run_case(&mut e, coord, coord, coord, coord, &format!("rlw rne[{k}]"));
    }

    let mut s = 0x6c0f_5050u32;
    for i in 0..3000u32 {
        // coordinates over the map, min at most max
        let a = frand(&mut s, -2000.0, 34000.0);
        let b = frand(&mut s, -2000.0, 34000.0);
        let c = frand(&mut s, -2000.0, 34000.0);
        let d = frand(&mut s, -2000.0, 34000.0);
        let (min_x, max_x) = (a.min(b), a.max(b));
        let (min_y, max_y) = (c.min(d), c.max(d));
        run_case(
            &mut e,
            max_x,
            min_x,
            max_y,
            min_y,
            &format!("rlw fuzz[{i}]"),
        );
    }
}

/// `0x680610` to its `ret` at `0x6807e9`, from the prologue so the frame and the stored presets run:
/// it has no inputs, and its 12 results are read at `0xc7d0d8`. A model that reloaded the
/// reciprocal as `f32` would differ in the last bit.
#[test]
fn scene_cull_planes_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
    e.run(0x680610, 0x6807e9);
    let bin: [f32; 12] = std::array::from_fn(|i| e.read_f32(CULL_SLOTS[i]));
    let ours = scene_cull_planes();
    let names = [
        "p0x", "p0y", "p0z", "p1x", "p1y", "p1z", "p2x", "p2y", "p2z", "p3x", "p3y", "p3z",
    ];
    for i in 0..12 {
        assert_f32(bin[i], ours[i], &format!("scene-cull {}", names[i]));
    }
}
