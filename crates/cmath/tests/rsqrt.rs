//! `RsqrtEstimate` (`0x456330`) against the exe, with the magic number the callers pass and with
//! others, over powers of two, a fuzz of positive values, and the zeros, NaN and infinities.

use benilla_re_cmath::rsqrt::rsqrt_estimate;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// `0x456330` with `ecx` = the magic and the value on the stack.
#[test]
fn fast_inv_sqrt_seed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x6669_7372_u32;

    let run = |emu: &mut Emu, magic: u32, x: f32| {
        let got = emu.call_f32(0x456330, magic, 0, &[x.to_bits()]);
        let ours = rsqrt_estimate(magic, x);
        assert_eq!(
            got.to_bits(),
            ours.to_bits(),
            "rsqrt({x}, {magic:#x}): binary={:#010x} ours={:#010x}",
            got.to_bits(),
            ours.to_bits()
        );
    };
    for x in [
        1.0f32,
        4.0,
        0.25,
        16.0,
        100.0,
        0.01,
        1e-3,
        1e6,
        2.0,
        0.5,
        255.0 * 255.0 * 3.0,
        0.0,
        -0.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::from_bits(1),
    ] {
        run(&mut emu, 0x5f39_97bb, x);
    }
    for _ in 0..400 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let x = (m * 1.0e6) as f32 + f32::from_bits(0x0000_0001);
        run(&mut emu, 0x5f39_97bb, x);
        run(&mut emu, next(&mut s), f32::from_bits(next(&mut s)));
    }
}
