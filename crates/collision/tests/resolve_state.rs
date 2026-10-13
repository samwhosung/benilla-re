//! `CMovement::LatchFallingFar` (`0x633240`), `ComputeTrianglePushback` (`0x634300`),
//! `CMovement::BoxTopTouchesTriangle` (`0x6333d0`), `CMovement::PlaneNearFootPoints` (`0x634960`)
//! and `CMovement::GetCollisionMask` (`0x6315f0`, with `IsUnitPlayerDriven` (`0x5fa550`) stubbed to
//! the chosen answer) run whole on a mover and a triangle list laid out at the client's offsets.

use benilla_re_collision::resolve_state::{ground_land_snap, reflect_velocity, LandSnapState};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

/// `0x633240` and `0x634300`.
#[test]
fn resolve_state_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x200);
    let hit_arr = emu.alloc(0x100);
    let holder = emu.alloc(0x10);
    let vel_va = emu.alloc(0x10);
    let out_va = emu.alloc(0x10);
    let mut s = 0x57a7_e001u32;
    let mut checked = 0u32;

    for case in 0..300u32 {
        let mut st = LandSnapState {
            flags: match case % 6 {
                0 => 0x2000,
                1 => 0x2000 | 0x4000,
                2 => 0x2000_0000,
                3 => 0,
                4 => 0x2000,
                _ => 0x2000 | 0x0800,
            },
            cur_z: fz(&mut s),
            fall_accum: if case % 5 == 0 {
                0x1f4 + (next(&mut s) & 0xff)
            } else {
                next(&mut s) % 0x1f4
            },
            fall_start_z: fz(&mut s),
            fall_vel: if case % 4 == 0 {
                -fz(&mut s).abs()
            } else {
                fz(&mut s).abs() + 0.01
            },
        };
        if case % 7 == 0 {
            st.fall_start_z = st.cur_z + f32::from_bits(0x3de3_8e39);
        }
        emu.write_u32(this_va + 0x40, st.flags);
        emu.write_u32(this_va + 0x18, st.cur_z.to_bits());
        emu.write_u32(this_va + 0x78, st.fall_accum);
        emu.write_u32(this_va + 0x7c, st.fall_start_z.to_bits());
        emu.write_u32(this_va + 0xa0, st.fall_vel.to_bits());
        emu.call(0x633240, this_va as u32, 0, &[]);
        let bin_flags = emu.read_u32(this_va + 0x40);
        let mut ours = st;
        ground_land_snap(&mut ours);
        assert_eq!(bin_flags, ours.flags, "case {case}: land-snap flags");

        let n = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let vel = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let scale = fz(&mut s);
        let idx = (next(&mut s) % 4) as u64;
        for (k, c) in n.iter().enumerate() {
            emu.write_u32(hit_arr + idx * 0x34 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(holder + 8, hit_arr as u32);
        for (k, c) in vel.iter().enumerate() {
            emu.write_u32(vel_va + (k as u64) * 4, c.to_bits());
        }
        emu.call(
            0x634300,
            0,
            0,
            &[
                out_va as u32,
                holder as u32,
                vel_va as u32,
                scale.to_bits(),
                idx as u32,
            ],
        );
        let ours = reflect_velocity(&n, &vel, scale);
        for k in 0..3u64 {
            let b = emu.read_f32(out_va + k * 4);
            assert_eq!(
                b.to_bits(),
                ours[k as usize].to_bits(),
                "case {case}: reflect[{k}]: binary={:#010x} ours={:#010x}",
                b.to_bits(),
                ours[k as usize].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("resolve_state: {checked} rounds bit-exact vs WoW.exe (2 functions per round)");
}

/// `0x6333d0` and `0x634960`.
#[test]
fn flush_probes_difftest() {
    use benilla_re_collision::resolve_state::{aabb_top_flush_test, prism_lower_flush_test};
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x100);
    let hits = emu.alloc(0x200);
    emu.write_u32(0xc4e534, hits as u32);
    let mut s = 0xf105_2026u32;
    let mut checked = 0u32;
    for case in 0..300u32 {
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = fz(&mut s).abs() + 0.2;
        let height = fz(&mut s).abs() + 0.5;

        let mut n = [fz(&mut s) * 0.1, fz(&mut s) * 0.1, 1.0f32];
        let mut d = fz(&mut s);
        if case % 3 == 0 {
            n = [0.0, 0.0, 1.0];
            d = -(pos[2] + height);
        }

        for (k, c) in pos.iter().enumerate() {
            emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(this_va + 0xb0, radius.to_bits());
        emu.write_u32(this_va + 0xb4, height.to_bits());
        let idx = (next(&mut s) % 4) as u64;
        for (k, c) in n.iter().enumerate() {
            emu.write_u32(hits + idx * 0x34 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(hits + idx * 0x34 + 0xc, d.to_bits());

        let eax = emu.call(0x6333d0, this_va as u32, 0, &[idx as u32]);
        let ours = aabb_top_flush_test(&pos, radius, height, &n, d);
        assert_eq!(eax & 0xff != 0, ours, "case {case}: aabb flush");

        let rec_va = hits + idx * 0x34;
        let eax = emu.call(0x634960, this_va as u32, 0, &[rec_va as u32]);
        let ours = prism_lower_flush_test(&pos, radius, &n, d);
        assert_eq!(eax & 0xff != 0, ours, "case {case}: prism flush");
        checked += 1;
    }
    eprintln!("flush_probes: {checked} rounds bit-exact vs WoW.exe");
}

/// `0x6315f0`.
#[test]
fn mask_select_difftest() {
    use benilla_re_collision::resolve_state::{query_mask_select, MaskSelectInput};
    let Some(wow) = Wow::load() else {
        return;
    };

    for pred in [false, true] {
        let mut emu = Emu::new(&wow);
        emu.stub(0x5fa550, pred as u32, 0);
        let this_va = emu.alloc(0x200);
        let unit_va = emu.alloc(0x1000);
        let desc_va = emu.alloc(0x20);
        let ptr2_va = emu.alloc(0x20);
        emu.write_u32(this_va + 0x15c, unit_va as u32);
        emu.write_u32(unit_va + 8, desc_va as u32);
        emu.write_u32(unit_va + 0xe68, ptr2_va as u32);
        let mut s = 0x3a5c_0001u32 ^ (pred as u32);
        for case in 0..120u32 {
            let inp = MaskSelectInput {
                flags: (next(&mut s) & 0x3fff_ffff) | if case % 3 == 0 { 0x1000_0000 } else { 0 },
                pitch: if case % 4 == 0 {
                    -0.6457718 + fz(&mut s) * 0.01
                } else {
                    fz(&mut s)
                },
                pred,
                unit_flag1: case % 2 == 0,
                unit_flag2: case % 5 != 1,
            };
            emu.write_u32(this_va + 0x40, inp.flags);
            emu.write_u32(this_va + 0x20, inp.pitch.to_bits());
            emu.write_u32(desc_va + 8, if inp.unit_flag1 { 0x10 } else { 0 });
            emu.write_u32(ptr2_va + 8, if inp.unit_flag2 { 0x10 } else { 0 });
            let eax = emu.call(0x6315f0, this_va as u32, 0, &[]);
            assert_eq!(
                eax,
                query_mask_select(&inp),
                "pred {pred} case {case}: mask"
            );
        }
    }
    eprintln!("mask_select: 240 rounds bit-exact vs WoW.exe");
}
