//! `IsNotFloorAt` (`0x633760`) runs whole with both its callees stubbed, `IsUnitPlayerDriven`
//! (`0x5fa550`) and `IsPointOverListedTriangle` (`0x6335d0`), so its own slope test is compared
//! alone: normals at the limit, either side of it and NaN, and random ones.

use benilla_re_collision::resolve_walk::walkable_hit;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

/// Both limits and both answers of the inside test.
#[test]
fn walkable_hit_difftest() {
    const COS50: f32 = f32::from_bits(0x3f24_8dbb);
    const COS80: f32 = f32::from_bits(0x3e31_d0d4);
    let Some(wow) = Wow::load() else {
        return;
    };

    let probes: &[f32] = &[
        COS50,
        COS80,
        f32::from_bits(COS50.to_bits() + 1),
        f32::from_bits(COS50.to_bits() - 1),
        f32::from_bits(COS80.to_bits() + 1),
        f32::from_bits(COS80.to_bits() - 1),
        f32::NAN,
        -f32::NAN,
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];

    for obj_pred in [false, true] {
        for in_face in [false, true] {
            let mut emu = Emu::new(&wow);
            emu.stub(0x5fa550, obj_pred as u32, 0);
            emu.stub(0x6335d0, in_face as u32, 0);
            let this_va = emu.alloc(0x20);
            let hits = emu.alloc(0x200);
            emu.write_u32(0xc4e534, hits as u32);

            let mut s = 0x6336_0001u32 ^ ((obj_pred as u32) << 1) ^ in_face as u32;
            let mut nzs: Vec<f32> = probes.to_vec();
            for _ in 0..400 {
                nzs.push(fz(&mut s));
                nzs.push(fz(&mut s) * 0.25);
                nzs.push(fz(&mut s) * 0.2 + 0.55);
            }

            for (i, &nz) in nzs.iter().enumerate() {
                let idx = (i as u64) % 4;
                let rv = hits + idx * 0x34;
                emu.write_u32(rv + 0x8, nz.to_bits());
                let al = emu.call(0x633760, this_va as u32, idx as u32, &[0]) & 0xff;
                let ours = walkable_hit(nz, obj_pred, in_face);
                assert_eq!(
                    al,
                    ours,
                    "obj_pred={obj_pred} in_face={in_face} nz={:#010x}: binary_al={al} ours={ours}",
                    nz.to_bits()
                );
            }
        }
    }
}
