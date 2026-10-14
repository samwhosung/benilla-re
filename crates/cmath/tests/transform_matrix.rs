//! `TranslateMatrix` (`0x7bdc40`), the 3×4 matrix applied to a vector (`0x7bcae0`) and the 4×4
//! rotation about an axis (`0x7bdb00`), run whole against `translate_matrix`,
//! `transform_mat43_vec3` and `axis_angle_mat4`. A NaN result matches any NaN.

use benilla_re_cmath::rotation::axis_angle_mat4;
use benilla_re_cmath::transform::{transform_mat43_vec3, translate_matrix};
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

const EDGE: [f32; 11] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    1e30,
    -1e30,
    f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

fn vectors(n: usize, seed: u32, fuzz: usize) -> Vec<Vec<f32>> {
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
                    let scale = [1.0, 10.0, 1e3, 1e-3][(next(&mut s) >> 8) as usize % 4];
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

/// `0x7bdc40` whole: `ecx` the matrix, changed in place, the translation on the stack. The twelve
/// entries other than row 3's first three are read back unchanged.
#[test]
fn translate_matrix_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let m = e.alloc(0x40);
    let t = e.alloc(0x10);
    let ms = vectors(16, 0x7bdc_4001, 1500);
    let ts = vectors(3, 0x7bdc_4002, 1500);
    for (k, mv) in ms.iter().enumerate() {
        let tv = &ts[(k * 3 + 1) % ts.len()];
        put(&mut e, m, mv);
        put(&mut e, t, tv);
        e.call(0x7bdc40, m as u32, 0, &[t as u32]);
        let mut ours: [f32; 16] = mv.as_slice().try_into().unwrap();
        translate_matrix(&mut ours, tv.as_slice().try_into().unwrap());
        for (i, o) in get(&e, m, 16).into_iter().enumerate() {
            assert_bits(o, ours[i], &format!("translate_matrix [{i}]"));
        }
    }
}

/// `0x7bcae0` whole: `ecx` the output, `edx` the matrix, the vector on the stack.
#[test]
fn transform_mat43_vec3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let m = e.alloc(0x30);
    let v = e.alloc(0x10);
    let out = e.alloc(0x10);
    let ms = vectors(12, 0x7bca_e001, 1500);
    let vs = vectors(3, 0x7bca_e002, 1500);
    for (k, mv) in ms.iter().enumerate() {
        let vv = &vs[(k * 3 + 1) % vs.len()];
        put(&mut e, m, mv);
        put(&mut e, v, vv);
        e.call(0x7bcae0, out as u32, m as u32, &[v as u32]);
        let ours = transform_mat43_vec3(
            mv.as_slice().try_into().unwrap(),
            vv.as_slice().try_into().unwrap(),
        );
        for (i, o) in get(&e, out, 3).into_iter().enumerate() {
            assert_bits(o, ours[i], &format!("transform_mat43_vec3 [{i}]"));
        }
    }
}

/// `0x7bdb00` whole: `ecx` the output, `edx` the axis, the angle and the unit flag on the stack.
/// Unit and non-unit axes, a zero axis (a NaN after the divide) and angles of both signs and
/// beyond a turn run.
#[test]
fn axis_angle_mat4_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let axis = e.alloc(0x10);
    let out = e.alloc(0x40);
    let axes = vectors(3, 0x7bdb_0001, 1500);
    let angles = [
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.0,
        std::f32::consts::PI,
        -std::f32::consts::PI,
        std::f32::consts::FRAC_PI_2,
        100.0,
        -1234.5,
    ];
    for (k, av) in axes.iter().enumerate() {
        for unit in [false, true] {
            let angle = angles[(k + usize::from(unit)) % angles.len()];
            put(&mut e, axis, av);
            e.call(
                0x7bdb00,
                out as u32,
                axis as u32,
                &[angle.to_bits(), u32::from(unit)],
            );
            let ours = axis_angle_mat4(av.as_slice().try_into().unwrap(), angle, unit);
            for (i, o) in get(&e, out, 16).into_iter().enumerate() {
                assert_bits(o, ours[i], &format!("axis_angle_mat4 [{i}] unit={unit}"));
            }
        }
    }
}
