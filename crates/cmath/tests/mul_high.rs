//! `MulHigh32` (`0x455c70`) against the exe, whole: the zeros, the ones, the extremes, the
//! operands at each power of two and a step either side, and a fuzz of pairs.

use benilla_re_cmath::scalar::mul_high32;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// `0x455c70` with `a` in `ecx` and `b` in `edx`, no stack arguments.
#[test]
fn mul_high32_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut run = |a: u32, b: u32| {
        let got = emu.call(0x455c70, a, b, &[]);
        assert_eq!(got, mul_high32(a, b), "MulHigh32({a:#x}, {b:#x})");
    };

    let mut edges = vec![
        0u32,
        1,
        2,
        3,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        u32::MAX - 1,
        u32::MAX,
    ];
    for k in 1..32 {
        let p = 1u32 << k;
        edges.extend([p - 1, p, p + 1]);
    }
    for &a in &edges {
        for &b in &edges {
            run(a, b);
        }
    }
    let mut s = 0x6d75_6c68_u32;
    for _ in 0..2000 {
        let a = next(&mut s);
        let b = next(&mut s);
        run(a, b);
    }
}
