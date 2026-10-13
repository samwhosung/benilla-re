//! `0x7c42f0` run whole against `rot2d`, over the quarter turns, both signs and angles within
//! eight turns, far inside the range `fsincos` reduces.

use benilla_re_cmath::rot2d::rot2d;
use benilla_re_difftest::{Emu, Wow};
use std::f32::consts::PI;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * (16.0 * PI) - 8.0 * PI
}

fn assert_bits(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}

#[test]
fn rot2d_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out_va = emu.alloc(0x20);
    let mut s = 0x3c1a_77e5u32;

    let mut angles: Vec<f32> = vec![
        0.0,
        PI,
        -PI,
        PI / 2.0,
        -PI / 2.0,
        PI / 4.0,
        -PI / 4.0,
        2.0 * PI,
        -2.0 * PI,
        1.0,
        -1.0,
        1.5,
        6.0,
        0.001,
        -0.001,
    ];
    for _ in 0..3000 {
        angles.push(fz(&mut s));
    }

    let mut asserts = 0usize;
    for (idx, &angle) in angles.iter().enumerate() {
        emu.call(0x7c42f0, out_va as u32, 0, &[angle.to_bits()]);
        let want = rot2d(angle);
        for (k, w) in want.iter().enumerate() {
            let got = emu.read_f32(out_va + (k as u64) * 4);
            assert_bits(got, *w, &format!("0x7c42f0 rot2d[{idx}].{k}"));
            asserts += 1;
        }
    }
    eprintln!(
        "rot2d: {asserts} bit-exact asserts over {} angles",
        angles.len()
    );
}
