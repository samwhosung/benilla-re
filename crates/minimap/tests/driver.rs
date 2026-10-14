//! The tile-window snap against the exe: the slice `0x6d95f8` to `0x6d96b1` of `0x6d93a0`, run in a
//! set-up `ebp` frame with the player position at `[ebp+8]` and the zoom index at `0x86f69c`; the
//! snapped window bounds `0xcea9c4` to `0xcea9d8` are read back.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_minimap::driver::minimap_update_snap;
use common::{assert_f32, fuzz, next};

const SNAP_BEGIN: u64 = 0x6d95f8;
const SNAP_END: u64 = 0x6d96b1; // the `call 0x40a280` after the `0xcea9d8` store
const ZOOM_L: u64 = 0x0086_f698 + 4; // 0x86f69c
const CEA9C4: u64 = 0x00ce_a9c4;

#[test]
fn minimap_update_snap_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let player = e.alloc(0x10);
    let ebp = frame + 0x80;

    let mut s: u32 = 0x6d95_0001;
    // NaN, the infinities and positions whose tile index passes `i32`: the `fistp` stores
    // `0x80000000`.
    const WIDE: [[f32; 3]; 7] = [
        [f32::NAN, 5.0, 1.0],
        [5.0, f32::NAN, 1.0],
        [f32::INFINITY, f32::NEG_INFINITY, 1.0],
        [1.0e30, -1.0e30, 1.0],
        [-1.0e12, 1.0e12, 1.0],
        [1.0e10, 100.0, 1.0],
        [100.0, -1.0e10, 1.0],
    ];
    for i in 0..6000u32 + WIDE.len() as u32 {
        let mut p = [
            fuzz(&mut s, 17000.0),
            fuzz(&mut s, 17000.0),
            fuzz(&mut s, 1000.0),
        ];
        if let Some(&x) = WIDE.get((i as usize).wrapping_sub(6000)) {
            p = x;
        }
        let zoom_l = (next(&mut s) >> 8) % 6;
        e.write_u32(player, p[0].to_bits());
        e.write_u32(player + 4, p[1].to_bits());
        e.write_u32(player + 8, p[2].to_bits());
        e.write_u32(ebp + 8, player as u32); // [ebp+8] = player ptr
        e.write_u32(ZOOM_L, zoom_l);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.set_reg(RegisterX86::FPCW, 0x027F);
        e.run(SNAP_BEGIN, SNAP_END);
        let bin_min = [
            e.read_f32(CEA9C4),
            e.read_f32(CEA9C4 + 4),
            e.read_f32(CEA9C4 + 8),
        ];
        let bin_max = [
            e.read_f32(CEA9C4 + 0xc),
            e.read_f32(CEA9C4 + 0x10),
            e.read_f32(CEA9C4 + 0x14),
        ];
        let w = minimap_update_snap(p, zoom_l);
        let ctx = format!("snap[{i}] p={p:?} zl={zoom_l}");
        for k in 0..3 {
            assert_f32(bin_min[k], w.min[k], &format!("{ctx} min[{k}]"));
            assert_f32(bin_max[k], w.max[k], &format!("{ctx} max[{k}]"));
        }
    }
}
