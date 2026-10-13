//! `CMovement::GetFallTime` (`0x7c5f50`) and `CMovement::GetApexTime` (`0x7c5da0`) run whole
//! through `call_f32`, with the terminal speeds as the image's `.data` holds them at `[0x87d894]`
//! and `[0x87d898]`.

use benilla_re_collision::resolve_fall::{fall_time_solve, swim_fall_vel};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

/// Random drops, speeds and flags, for both functions.
#[test]
fn fall_physics_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x100);
    let mut s = 0xfa11_0001u32;
    let vmax_land = emu.read_f32(0x87d894);
    let vmax_swim = emu.read_f32(0x87d898);
    let mut checked = 0u32;
    for case in 0..400u32 {
        let fa0 = match case % 5 {
            0 => 0.0,
            1 => 1e-9,
            _ => fz(&mut s).abs() * 20.0,
        };
        let flags = (if case % 3 == 0 { 0x2000_0000 } else { 0 })
            | (if case % 2 == 0 { 0x2000 } else { 0 });
        let d = if case % 7 == 0 {
            -fz(&mut s).abs()
        } else {
            fz(&mut s).abs() * 50.0
        };
        let flag = (case % 4 == 1) as u32;
        emu.write_u32(this_va + 0xa0, fa0.to_bits());
        emu.write_u32(this_va + 0x40, flags);
        let bt = emu.call_f32(0x7c5f50, this_va as u32, 0, &[d.to_bits(), flag]);
        let ot = fall_time_solve(fa0, flags, vmax_land, vmax_swim, d, flag) as f32;
        assert_eq!(
            bt.to_bits(),
            ot.to_bits(),
            "case {case}: solve binary={:#010x} ours={:#010x}",
            bt.to_bits(),
            ot.to_bits()
        );

        let bv = emu.call_f32(0x7c5da0, this_va as u32, 0, &[]);
        let ov = swim_fall_vel(fa0, flags) as f32;
        assert_eq!(bv.to_bits(), ov.to_bits(), "case {case}: swimvel");
        checked += 1;
    }
    eprintln!("fall_physics: {checked} rounds bit-exact vs WoW.exe (2 functions)");
}
