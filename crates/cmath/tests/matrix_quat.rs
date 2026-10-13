//! `0x7c0190` run whole against `matrix_to_quat`. The matrices come from unit quaternions, so
//! every square root has a non-negative argument: the identity and small rotations take the trace
//! path, rotations near 180° about each axis the largest-diagonal path with `i` = 0, 1 and 2, and
//! 120° about (1, 1, 1) puts the trace at the branch.

use benilla_re_cmath::quaternion::matrix_to_quat;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 2.0 - 1.0
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

fn unit_quat(s: &mut u32) -> [f32; 4] {
    let q = [fz(s), fz(s), fz(s), fz(s)];
    let n = ((q[0] as f64).powi(2)
        + (q[1] as f64).powi(2)
        + (q[2] as f64).powi(2)
        + (q[3] as f64).powi(2))
    .sqrt();
    if n < 1.0e-12 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [
        (q[0] as f64 / n) as f32,
        (q[1] as f64 / n) as f32,
        (q[2] as f64 / n) as f32,
        (q[3] as f64 / n) as f32,
    ]
}

fn rot_matrix(q: [f32; 4]) -> [f32; 9] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    [
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y - w * z),
        2.0 * (x * z + w * y),
        2.0 * (x * y + w * z),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z - w * x),
        2.0 * (x * z - w * y),
        2.0 * (y * z + w * x),
        1.0 - 2.0 * (x * x + y * y),
    ]
}

#[test]
fn matrix_quat_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let m_va = emu.alloc(0x40);
    let out_va = emu.alloc(0x20);
    let mut s = 0x6b1f_3d27u32;

    let mut mats: Vec<[f32; 9]> = vec![
        rot_matrix([0.0, 0.0, 0.0, 1.0]),
        rot_matrix([1.0, 0.0, 0.0, 0.0]),
        rot_matrix([0.0, 1.0, 0.0, 0.0]),
        rot_matrix([0.0, 0.0, 1.0, 0.0]),
        rot_matrix([0.70710677, 0.0, 0.0, 0.70710677]),
        rot_matrix([0.0, 0.70710677, 0.0, 0.70710677]),
        rot_matrix([0.0, 0.0, 0.70710677, 0.70710677]),
        rot_matrix([0.5, 0.5, 0.5, 0.5]),
        rot_matrix([-0.5, -0.5, -0.5, 0.5]),
    ];

    for _ in 0..1500 {
        mats.push(rot_matrix(unit_quat(&mut s)));

        let tiny_w = [fz(&mut s), fz(&mut s), fz(&mut s), fz(&mut s) * 1.0e-2];
        let n = ((tiny_w[0] as f64).powi(2)
            + (tiny_w[1] as f64).powi(2)
            + (tiny_w[2] as f64).powi(2)
            + (tiny_w[3] as f64).powi(2))
        .sqrt();
        if n >= 1.0e-12 {
            mats.push(rot_matrix([
                (tiny_w[0] as f64 / n) as f32,
                (tiny_w[1] as f64 / n) as f32,
                (tiny_w[2] as f64 / n) as f32,
                (tiny_w[3] as f64 / n) as f32,
            ]));
        }
    }

    let mut asserts = 0usize;
    for (idx, m) in mats.iter().enumerate() {
        for (i, v) in m.iter().enumerate() {
            emu.write_u32(m_va + (i as u64) * 4, v.to_bits());
        }
        emu.call(0x7c0190, out_va as u32, 0, &[m_va as u32]);
        let want = matrix_to_quat(m);
        for (k, w) in want.iter().enumerate() {
            let got = emu.read_f32(out_va + (k as u64) * 4);
            assert_bits(got, *w, &format!("0x7c0190 matrix_to_quat[{idx}].{k}"));
            asserts += 1;
        }
    }
    eprintln!(
        "matrix_quat: {asserts} bit-exact asserts over {} matrices",
        mats.len()
    );
}
