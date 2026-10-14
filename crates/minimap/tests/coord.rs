//! The world and tile coordinate maps (`0x6da5b0`, `0x6da540`) against the exe, each called whole;
//! both write through a pointer.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_minimap::coord::{tile_to_world, world_to_tile};
use common::{assert_f32, assert_i32, fuzz, fuzz_i32, g, next};

const TILE_TO_WORLD: u64 = 0x6da5b0; // __fastcall(out* ecx, tile* edx)
const WORLD_TO_TILE: u64 = 0x6da540; // __fastcall(out* ecx, world* edx)

#[test]
fn tile_to_world_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x10);
    let tile = e.alloc(0x10);
    let mut s: u32 = 0x6da5_0001;
    for i in 0..6000u32 {
        let t = [fuzz_i32(&mut s, 4096), fuzz_i32(&mut s, 4096)];
        let origin_y = fuzz(&mut s, 40000.0);
        e.write_u32(g::ORIGIN_Y, origin_y.to_bits());
        e.write_u32(tile, t[0] as u32);
        e.write_u32(tile + 4, t[1] as u32);
        e.write(out, &[0u8; 12]);
        e.call(TILE_TO_WORLD, out as u32, tile as u32, &[]);
        let binv = [e.read_f32(out), e.read_f32(out + 4), e.read_f32(out + 8)];
        let ours = tile_to_world(t, origin_y);
        for (k, (&b, &o)) in binv.iter().zip(&ours).enumerate() {
            assert_f32(
                b,
                o,
                &format!("tile_to_world[{i}] tile={t:?} oy={origin_y} out[{k}]"),
            );
        }
    }
}

#[test]
fn world_to_tile_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x10);
    let world = e.alloc(0x10);
    let mut s: u32 = 0x6da4_0001;
    // NaN, the infinities and coordinates whose tile index passes `i32` and `i64`: `__ftol` returns
    // the low dword of its qword, 0 for the indefinite.
    const WIDE: [[f32; 2]; 8] = [
        [f32::NAN, 5.0],
        [5.0, f32::NAN],
        [f32::INFINITY, f32::NEG_INFINITY],
        [1.0e30, -1.0e30],
        [-1.0e12, 1.0e12],
        [1.0e13, -1.0e13],
        [4.0e12, 2.0e18],
        [1.0e19, 1.0e19],
    ];
    for i in 0..6000u32 + WIDE.len() as u32 {
        // Full map range incl. negative; high bits → wide coverage of the floor-branch.
        let mut w = [fuzz(&mut s, 40000.0), fuzz(&mut s, 40000.0)];
        if let Some(&x) = WIDE.get((i as usize).wrapping_sub(6000)) {
            w = x;
        }
        e.write_u32(world, w[0].to_bits());
        e.write_u32(world + 4, w[1].to_bits());
        e.write(out, &[0u8; 8]);
        e.call(WORLD_TO_TILE, out as u32, world as u32, &[]);
        let binv = [e.read_u32(out) as i32, e.read_u32(out + 4) as i32];
        let ours = world_to_tile(w);
        assert_i32(
            binv[0],
            ours[0],
            &format!("world_to_tile[{i}] w={w:?} out[0]"),
        );
        assert_i32(
            binv[1],
            ours[1],
            &format!("world_to_tile[{i}] w={w:?} out[1]"),
        );
        let _ = next(&mut s);
    }
}
