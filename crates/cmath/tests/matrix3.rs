//! `Matrix3Determinant` (`0x7be0e0`), `Matrix3Adjugate` (`0x7be1e0`) and `InvertMatrix3`
//! (`0x7be2a0`) run whole against `det3b`, `adj3t` and `inv3`. The matrices are each edge value
//! filled across all nine elements (zeros, ±1, ±0.5, ±1e30, the smallest normal and subnormal,
//! ±`f32::MAX`, ±∞), a mix of them, and 1500 random non-NaN bit patterns. A NaN result matches any
//! NaN: the x87 indefinite and the host's NaN differ only in sign and payload.

use benilla_re_cmath::matrix3;
use benilla_re_difftest::{Emu, Wow};

/// Nine-float inputs: the edge fills, the mix and the random patterns.
fn synth_9() -> Vec<[f32; 9]> {
    let edges = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        123.456,
        -789.012,
    ];
    let mut v: Vec<[f32; 9]> = edges.iter().map(|&e| [e; 9]).collect();
    v.push(core::array::from_fn(|i| edges[i % edges.len()]));
    let mut s: u32 = 0x1234_5678;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..1500 {
        v.push(core::array::from_fn(|_| {
            let mut f = f32::from_bits(next());
            while f.is_nan() {
                f = f32::from_bits(next());
            }
            f
        }));
    }
    v
}

/// Determinants for `InvertMatrix3`: edges and 400 random non-NaN bit patterns.
fn synth_scalars() -> Vec<f32> {
    let mut v = vec![
        0.0f32, -0.0, 1.0, -1.0, 0.5, 2.0, 1e30, -1e-30, 123.456, -789.0,
    ];
    let mut s: u32 = 0x9e37_79b9;
    for _ in 0..400 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let f = f32::from_bits(s);
        if !f.is_nan() {
            v.push(f);
        }
    }
    v
}

fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// An emulator with the matrix at `a` and the output at `dst`.
fn setup(wow: &Wow) -> (Emu<'_>, u64, u64) {
    let mut emu = Emu::new(wow);
    let dst = emu.alloc(0x80);
    let a = emu.alloc(0x80);
    (emu, dst, a)
}

fn put(emu: &mut Emu, base: u64, m: &[f32]) {
    for (i, &f) in m.iter().enumerate() {
        emu.write_u32(base + i as u64 * 4, f.to_bits());
    }
}

fn read9(emu: &Emu, base: u64) -> [f32; 9] {
    core::array::from_fn(|i| emu.read_f32(base + i as u64 * 4))
}

#[test]
fn det3b() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, _, a) = setup(&wow);
    let mats = synth_9();
    for m in &mats {
        put(&mut emu, a, m);
        let got = emu.call_f32(0x7be0e0, a as u32, 0, &[]);
        assert_f32(got, matrix3::det3b(m), "det3b");
    }
    eprintln!("det3b: {} cases bit-exact", mats.len());
}

#[test]
fn adj3t() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, dst, a) = setup(&wow);
    let mats = synth_9();
    for m in &mats {
        put(&mut emu, a, m);
        emu.call(0x7be1e0, a as u32, 0, &[dst as u32]);
        let want = matrix3::adj3t(m);
        for (i, (&b, &o)) in read9(&emu, dst).iter().zip(want.iter()).enumerate() {
            assert_f32(b, o, &format!("adj3t[{i}]"));
        }
    }
    eprintln!("adj3t: {} cases bit-exact", mats.len());
}

#[test]
fn inv3() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, dst, a) = setup(&wow);
    let mats = synth_9();
    let scalars = synth_scalars();
    for (k, m) in mats.iter().enumerate() {
        let s = scalars[k % scalars.len()];
        put(&mut emu, a, m);
        emu.call(0x7be2a0, a as u32, 0, &[dst as u32, s.to_bits()]);
        let want = matrix3::inv3(m, s);
        for (i, (&b, &o)) in read9(&emu, dst).iter().zip(want.iter()).enumerate() {
            assert_f32(b, o, &format!("inv3[{i}]"));
        }
    }
    eprintln!("inv3: {} cases bit-exact", mats.len());
}
