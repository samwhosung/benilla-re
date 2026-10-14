//! The matrix products `0x7bdfc0` and `0x7bc6a0` and the dot leaf `0x7bc8c0`, run whole against
//! `multiply_matrix3`, `multiply_matrix4` and `dot_leaf` on edge matrices (zeros of both signs,
//! the infinities, NaN) and a fuzz. A NaN result matches any NaN.

use benilla_re_cmath::multiply::{dot_leaf, multiply_matrix3, multiply_matrix4};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

fn assert_bits(binary: f32, ours: f32, what: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

const EDGE: [f32; 12] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    1e30,
    -1e30,
    f32::MIN_POSITIVE,
    f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// `n` values: each edge value across the matrix, the edges in turn along it, and a fuzz of
/// finite values over six decades.
fn matrices(n: usize, seed: u32, fuzz: usize) -> Vec<Vec<f32>> {
    let mut out: Vec<Vec<f32>> = EDGE.iter().map(|&e| vec![e; n]).collect();
    for shift in 0..EDGE.len() {
        out.push((0..n).map(|i| EDGE[(i + shift) % EDGE.len()]).collect());
    }
    let mut s = seed;
    for _ in 0..fuzz {
        out.push(
            (0..n)
                .map(|_| {
                    let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                    let scale = [1.0, 10.0, 1e3, 1e-3, 1e6][(next(&mut s) >> 8) as usize % 5];
                    ((m * 2.0 - 1.0) * scale) as f32
                })
                .collect(),
        );
    }
    out
}

fn put(e: &mut Emu, va: u64, v: &[f32]) {
    for (i, x) in v.iter().enumerate() {
        e.write_u32(va + 4 * i as u64, x.to_bits());
    }
}

fn get(e: &Emu, va: u64, n: usize) -> Vec<f32> {
    (0..n).map(|i| e.read_f32(va + 4 * i as u64)).collect()
}

/// `0x7bc8c0` whole: `ecx` the vector, `edx` the 4×4, the result in `st0`.
#[test]
fn dot_leaf_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let a = e.alloc(0x10);
    let b = e.alloc(0x40);
    let avs = matrices(4, 0x7bc8_c001, 1500);
    let bvs = matrices(16, 0x7bc8_c002, 1500);
    for (k, av) in avs.iter().enumerate() {
        let bv = &bvs[(k * 7 + 1) % bvs.len()];
        put(&mut e, a, av);
        put(&mut e, b, bv);
        let bin = e.call_f32(0x7bc8c0, a as u32, b as u32, &[]);
        let ours = dot_leaf(
            av.as_slice().try_into().unwrap(),
            bv.as_slice().try_into().unwrap(),
        );
        assert_bits(bin, ours, "dot_leaf");
    }
}

/// `0x7bdfc0` whole: `ecx` the output, `edx` the left matrix, the right one on the stack.
#[test]
fn multiply_matrix3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let a = e.alloc(0x30);
    let b = e.alloc(0x30);
    let out = e.alloc(0x30);
    let avs = matrices(9, 0x7bdf_c001, 1500);
    let bvs = matrices(9, 0x7bdf_c002, 1500);
    for (k, av) in avs.iter().enumerate() {
        let bv = &bvs[(k * 5 + 1) % bvs.len()];
        put(&mut e, a, av);
        put(&mut e, b, bv);
        e.call(0x7bdfc0, out as u32, a as u32, &[b as u32]);
        let ours = multiply_matrix3(
            av.as_slice().try_into().unwrap(),
            bv.as_slice().try_into().unwrap(),
        );
        for (i, o) in get(&e, out, 9).into_iter().enumerate() {
            assert_bits(o, ours[i], &format!("multiply_matrix3 [{i}]"));
        }
    }
}

/// `0x7bc6a0` whole: `ecx` the output, `edx` the left matrix, the right one on the stack.
#[test]
fn multiply_matrix4_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let a = e.alloc(0x40);
    let b = e.alloc(0x40);
    let out = e.alloc(0x40);
    let avs = matrices(16, 0x7bc6_a001, 1500);
    let bvs = matrices(16, 0x7bc6_a002, 1500);
    for (k, av) in avs.iter().enumerate() {
        let bv = &bvs[(k * 5 + 1) % bvs.len()];
        put(&mut e, a, av);
        put(&mut e, b, bv);
        e.call(0x7bc6a0, out as u32, a as u32, &[b as u32]);
        let ours = multiply_matrix4(
            av.as_slice().try_into().unwrap(),
            bv.as_slice().try_into().unwrap(),
        );
        for (i, o) in get(&e, out, 16).into_iter().enumerate() {
            assert_bits(o, ours[i], &format!("multiply_matrix4 [{i}]"));
        }
    }
}
