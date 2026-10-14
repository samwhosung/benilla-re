//! `CosineInterpolate` (`0x5b7bb0`), run whole: three `f32` on the stack and the result left in
//! `st0` at 53 bits. The inputs are every pair of twelve edge values at five weights, and a fuzz in
//! [−2000, 2000]; the `fcos` runs through the host's cosine, which the emulator uses too.

mod common;

use benilla_re_cmath::interp;
use benilla_re_difftest::Emu;
use common::*;

fn interp_inputs() -> Vec<(f32, f32, f32)> {
    let edges = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, -0.5, 0.25, 0.999_999, 1e3, -1e3, 123.456, -789.012,
    ];
    let mut v = Vec::new();
    for &a in &edges {
        for &b in &edges {
            for &t in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
                v.push((a, b, t));
            }
        }
    }
    let mut s = 0x5b7b_9000u32;
    let next = |s: &mut u32| {
        *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        *s
    };
    let f = |s: &mut u32| -2000.0 + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 4000.0;
    for _ in 0..5000 {
        v.push((f(&mut s) as f32, f(&mut s) as f32, f(&mut s) as f32));
    }
    v
}

#[test]
fn cmath_cosine_smoothstep() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut n = 0u32;
    for (a, b, t) in interp_inputs() {
        let got = emu.call_f64(0x5b7bb0, 0, 0, &[a.to_bits(), b.to_bits(), t.to_bits()]);
        assert_eq!(
            got.to_bits(),
            interp::cosine_smoothstep(a, b, t).to_bits(),
            "cosine_smoothstep a={a} b={b} t={t}"
        );
        n += 1;
    }
    eprintln!("cmath_cosine_smoothstep: {n} cases bit-exact vs WoW.exe @ 0x5b7bb0");
}
