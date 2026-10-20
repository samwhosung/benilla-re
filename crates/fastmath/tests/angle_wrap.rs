//! `0x455b30` run against `angle_wrap`: the wrap into `[0, 2π)` of the remainder `__CIfmod`
//! (`0x73f90a`) returns.
//!
//! The first part runs the whole function, the `fmod` included, on finite angles of at most 1e9 in
//! size: zeros of both signs, the smallest values, one turn and multiples of it with a step either
//! side, and a fuzz. Two limits of the emulator are kept out of it. Its `fprem` is exact only while
//! the quotient times 2π fits 53 bits, which holds to about 5e8 turns; and a remainder of exactly
//! zero is +0 there where the CPU gives the dividend's sign (Intel SDM, `FPREM`), so a zero remainder
//! of a nonzero angle is compared as a zero. An infinite or NaN angle takes the CRT's error path,
//! which reads CRT state the emulator does not hold, and is not run.
//!
//! The second part replaces `0x73f90a` by a chosen value and runs the wrap on it: remainders the
//! first part cannot produce (NaN, the infinities, and `0` to `1e-300` below it) go through the
//! compare with 0 and the add of 2π.
//!
//! Host double (`docs/METHOD.md#the-harness`): `angle_wrap_diff` runs `fprem` at `0x73f91f`. The
//! emulator computes each in host `double`, so where its result reaches a compared value the host's
//! math stands in for the x87's, and the test shows the code around it.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_fastmath::angle_wrap::angle_wrap;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

fn assert_f64(binary: f64, ours: f64, what: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{what}: binary={:#018x} ours={:#018x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

#[test]
fn angle_wrap_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // Whole function.
    let run = |emu: &mut Emu, a: f32| {
        let got = emu.call_f64(0x455b30, 0, 0, &[a.to_bits()]);
        let ours = angle_wrap(a);
        if got == 0.0 && ours == 0.0 && a != 0.0 {
            return;
        }
        assert_f64(got, ours, &format!("angle_wrap({a})"));
    };
    let tau = f32::from_bits(0x40c9_0fdb);
    let mut edges = vec![
        0.0f32,
        -0.0,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0,
        -1.0,
        std::f32::consts::PI,
        -std::f32::consts::PI,
        100.0,
        -1234.5,
        1e9,
        -1e9,
    ];
    for k in (-4..=4).filter(|&k| k != 0) {
        let turns = tau * k as f32;
        edges.extend([
            turns,
            f32::from_bits(turns.to_bits() + 1),
            f32::from_bits(turns.to_bits().wrapping_sub(1)),
        ]);
    }
    for a in edges {
        run(&mut emu, a);
    }
    let mut s = 0x455b_3001u32;
    for _ in 0..10000 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(&mut emu, ((m * 2.0 - 1.0) * 7.0) as f32);
    }
    for _ in 0..20000 {
        let a = f32::from_bits(next(&mut s));
        if a.is_finite() && a.abs() <= 1e9 {
            run(&mut emu, a);
        }
    }

    // `0x73f90a` replaced by a chosen remainder, in a second emulator (a hook added after the
    // block ran is not seen).
    let mut emu = Emu::new(&wow);
    let slot = emu.alloc(8);
    emu.stub_f64(0x73f90a, slot, 0);
    let two_pi = f64::from(tau);
    let inject = |emu: &mut Emu, r: f64| {
        emu.write(slot, &r.to_bits().to_le_bytes());
        let got = emu.call_f64(0x455b30, 0, 0, &[0]);
        let want = if r < 0.0 { r + two_pi } else { r };
        assert_f64(got, want, &format!("wrap of remainder {r}"));
    };
    for r in [
        0.0,
        -0.0,
        f64::from_bits(0x8000_0000_0000_0001),
        -1e-300,
        -1.0,
        1.0,
        -std::f64::consts::PI,
        two_pi,
        -two_pi,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        inject(&mut emu, r);
    }
    for _ in 0..10000 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        inject(&mut emu, (m * 2.0 - 1.0) * 7.0);
    }
}
