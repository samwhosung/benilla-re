//! `0x454f40` whole, against `quadratic_roots`: the roots are poisoned, the function runs, and `al`
//! and both outputs are compared bit for bit. The discriminant at `h == g` (no roots), NaN and the
//! infinities in each position, `b` of both signs and zeros, `a` of 0 (an infinite `inv`) and the
//! tie `r_b == r_a` run among a fuzz, a second edge set against c of ±1, and a bounded fuzz. A NaN
//! output matches any NaN.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_fastmath::quadratic::quadratic_roots;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

fn assert_bits(binary: u32, ours: u32, what: &str) {
    if f32::from_bits(binary).is_nan() && f32::from_bits(ours).is_nan() {
        return;
    }
    assert_eq!(
        binary, ours,
        "{what}: binary={binary:#010x} ours={ours:#010x}"
    );
}

#[test]
fn quadratic_roots_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let t1 = emu.alloc(8);
    let t2 = emu.alloc(8);
    const POISON: u32 = 0xdead_beef;

    let mut run = |a: f32, b: f32, c: f32| {
        emu.write_u32(t1, POISON);
        emu.write_u32(t2, POISON);
        let eax = emu.call(
            0x454f40,
            t1 as u32,
            t2 as u32,
            &[a.to_bits(), b.to_bits(), c.to_bits()],
        );
        let ctx = format!("a={a} b={b} c={c}");
        match quadratic_roots(a, b, c) {
            None => {
                assert_eq!(eax & 0xff, 0, "{ctx}: al");
                assert_eq!(emu.read_u32(t1), POISON, "{ctx}: t1 untouched");
                assert_eq!(emu.read_u32(t2), POISON, "{ctx}: t2 untouched");
            }
            Some((o1, o2)) => {
                assert_eq!(eax & 0xff, 1, "{ctx}: al");
                assert_bits(emu.read_u32(t1), o1.to_bits(), &format!("{ctx}: t1"));
                assert_bits(emu.read_u32(t2), o2.to_bits(), &format!("{ctx}: t2"));
            }
        }
    };

    let edges = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        2.0,
        0.5,
        -0.5,
        1e-30,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &a in &edges {
        for &b in &edges {
            for &c in &edges {
                run(a, b, c);
            }
        }
    }

    // h == g exactly: b² = 4ac, for a power-of-two triple, and the roots at the tie r_b == r_a.
    for &(a, b, c) in &[
        (1.0f32, 2.0f32, 1.0f32),
        (1.0, -2.0, 1.0),
        (2.0, 4.0, 2.0),
        (0.25, 1.0, 1.0),
        (1.0, 4.0, 3.0),
        (1.0, -4.0, 3.0),
        (1.0, 0.0, -1.0),
        (1.0, -0.0, -1.0),
        (3.0, 0.0, -3.0),
    ] {
        run(a, b, c);
        run(-a, -b, -c);
    }

    let mut s = 0x7175_6164u32;
    for _ in 0..4000 {
        let m = |s: &mut u32| f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        let sign = |s: &mut u32| if next(s) >> 31 == 0 { 1.0 } else { -1.0 };
        let a = (sign(&mut s) * m(&mut s) * 10.0) as f32;
        let b = (sign(&mut s) * m(&mut s) * 10.0) as f32;
        let c = (sign(&mut s) * m(&mut s) * 10.0) as f32;
        run(a, b, c);
        run(a, b, -a.abs() * 0.5 + f32::MIN_POSITIVE);
    }

    // A second edge set, with a subnormal, 2 and 3 and two ordinary values, against c of ±1.
    let edges2 = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        3.0,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        123.456,
        -789.012,
    ];
    for &a in &edges2 {
        for &b in &edges2 {
            run(a, b, 1.0);
            run(a, b, -1.0);
        }
    }
    for _ in 0..8000 {
        let m = |s: &mut u32| f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        run(
            ((m(&mut s) * 2.0 - 1.0) * 10.0) as f32,
            ((m(&mut s) * 2.0 - 1.0) * 10.0) as f32,
            ((m(&mut s) * 2.0 - 1.0) * 10.0) as f32,
        );
    }
}
