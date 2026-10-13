//! The float part of `RandomUnitVector2` (`0x44fd00`), run as the slice `0x44fdc8` to `0x44fe17`
//! with the accumulator in `esi`: the angle from the mantissa, then `fsincos`. The slice also
//! writes the state through `ecx`, to a scratch cell.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_random::rand2_gradient;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn rand_44fd00_gradient() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40;
    let stack = emu.alloc(0x400);
    let state = emu.alloc(0x10); // the state the slice writes, unchecked
    let grad = emu.alloc(0x10); // the (cos, sin) output, through [ebp-0x18]

    let run = |emu: &mut Emu, hash: u32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x300);
        emu.set_reg(RegisterX86::ESI, u64::from(hash)); // the accumulator
        emu.set_reg(RegisterX86::ECX, state);
        emu.set_reg(RegisterX86::EAX, 0);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.write_u32(ebp - 0x18, grad as u32);
        emu.run(0x44fdc8, 0x44fe17);
        (emu.read_f32(grad), emu.read_f32(grad + 4))
    };

    let mut total = 0u32;
    // Mantissas giving the angles 0, π, just under 2π, and quarter and eighth turns.
    for &h in &[
        0x0000_0000u32,
        0x0040_0000,
        0x007f_ffff,
        0x0020_0000,
        0x0060_0000,
        0x0010_0000,
        0xffff_ffff,
        0x1234_5678,
        0x8000_0001,
        0x5555_5555,
    ] {
        let (c, s) = run(&mut emu, h);
        let (wc, ws) = rand2_gradient(h);
        assert_eq!(c.to_bits(), wc.to_bits(), "cos hash={h:#x}");
        assert_eq!(s.to_bits(), ws.to_bits(), "sin hash={h:#x}");
        total += 2;
    }
    let mut st = 0x44fd_0000u32;
    for _ in 0..5000 {
        let h = next(&mut st);
        let (c, s) = run(&mut emu, h);
        let (wc, ws) = rand2_gradient(h);
        assert_eq!(c.to_bits(), wc.to_bits(), "cos fuzz hash={h:#x}");
        assert_eq!(s.to_bits(), ws.to_bits(), "sin fuzz hash={h:#x}");
        total += 2;
    }
    eprintln!("rand_44fd00_gradient: {total} bit-exact (mantissa-trick → 2π → fsincos via libm)");
}
