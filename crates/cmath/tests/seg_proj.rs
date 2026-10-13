//! `0x7c46d0` run whole through `call_f64`, which reads `st0` unrounded, against `seg_proj`: the
//! squared distance and `out_t`. The cases cover each of the three branches, both ends exactly, a
//! zero-length segment and NaN. A NaN result matches any NaN: the payload x87 carries through
//! differs from the host's.

use benilla_re_cmath::seg_proj::seg_proj;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn fz(s: &mut u32, scale: f32) -> f32 {
    (((next(s) >> 9) as f32 / (1u32 << 23) as f32) * 2.0 - 1.0) * scale
}

fn assert_f64(binary: f64, ours: f64, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: distSq binary={binary} ({:#018x}) ours={ours} ({:#018x})",
        binary.to_bits(),
        ours.to_bits()
    );
}

fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: out_t binary={binary} ({:#010x}) ours={ours} ({:#010x})",
        binary.to_bits(),
        ours.to_bits()
    );
}

#[test]
fn seg_proj_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let seg_va = emu.alloc(0x40);
    let q_va = emu.alloc(0x20);
    let out_t_va = emu.alloc(0x10);

    let seg_x = [0.0f32, 0.0, 0.0, 2.0, 0.0, 0.0];
    let mut cases: Vec<([f32; 6], [f32; 3])> = vec![
        (seg_x, [1.0, 0.0, 0.0]),
        (seg_x, [1.0, 1.0, 0.0]),
        (seg_x, [1.0, 0.0, 1.0]),
        (seg_x, [0.5, 0.3, -0.7]),
        (seg_x, [1.7, -2.0, 0.5]),
        (seg_x, [0.001, 5.0, 0.0]),
        (seg_x, [1.999, 5.0, 0.0]),
        (seg_x, [-0.5, 0.0, 0.0]),
        (seg_x, [-1.0, 2.0, 0.0]),
        (seg_x, [0.0, 0.0, 0.0]),
        (seg_x, [-3.0, -4.0, 0.0]),
        (seg_x, [-0.0, 1.0, 0.0]),
        (seg_x, [2.5, 0.0, 0.0]),
        (seg_x, [3.0, 1.0, 0.0]),
        (seg_x, [2.0, 0.0, 0.0]),
        (seg_x, [2.0, 3.0, 0.0]),
        (seg_x, [10.0, 10.0, 10.0]),
        (seg_x, [-5.0, 0.0, 0.0]),
        (seg_x, [7.0, 0.0, 0.0]),
        ([1.0, 2.0, 3.0, 4.0, 6.0, 9.0], [2.0, 3.0, 4.0]),
        ([1.0, 2.0, 3.0, 4.0, 6.0, 9.0], [-1.0, -1.0, -1.0]),
        ([1.0, 2.0, 3.0, 4.0, 6.0, 9.0], [10.0, 10.0, 10.0]),
        ([1.0, 2.0, 3.0, 4.0, 6.0, 9.0], [2.5, 4.0, 6.0]),
        ([3.0, 3.0, 3.0, 3.0, 3.0, 3.0], [3.0, 3.0, 3.0]),
        ([3.0, 3.0, 3.0, 3.0, 3.0, 3.0], [5.0, 1.0, 0.0]),
        ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [1.0, 2.0, 2.0]),
        ([0.0, 0.0, 0.0, 1e-6, 0.0, 0.0], [0.5e-6, 1.0, 0.0]),
        ([0.0, 0.0, 0.0, 1e-6, 0.0, 0.0], [-1.0, 0.0, 0.0]),
        (
            [1000.0, -2000.0, 500.0, 1100.0, -1900.0, 600.0],
            [1050.0, -1950.0, 550.0],
        ),
        (
            [16000.0, 16000.0, 0.0, -16000.0, -16000.0, 0.0],
            [100.0, -100.0, 0.0],
        ),
        ([-0.0, -0.0, -0.0, 1.0, 0.0, 0.0], [0.5, 0.0, 0.0]),
        (seg_x, [f32::NAN, 0.0, 0.0]),
        (seg_x, [0.0, f32::NAN, 0.0]),
        ([f32::NAN, 0.0, 0.0, 2.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([0.0, 0.0, 0.0, f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]),
        (seg_x, [f32::INFINITY, 0.0, 0.0]),
        (seg_x, [0.0, f32::INFINITY, 0.0]),
        (seg_x, [f32::NEG_INFINITY, 0.0, 0.0]),
        ([f32::INFINITY, 0.0, 0.0, 2.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
    ];

    let mut s = 0x1357_acefu32;
    for _ in 0..2000 {
        let a = [fz(&mut s, 50.0), fz(&mut s, 50.0), fz(&mut s, 50.0)];

        let elen = match next(&mut s) % 4 {
            0 => 0.01,
            1 => 1.0,
            2 => 20.0,
            _ => 200.0,
        };
        let b = [
            a[0] + fz(&mut s, elen),
            a[1] + fz(&mut s, elen),
            a[2] + fz(&mut s, elen),
        ];

        let tt = fz(&mut s, 1.5) * 0.5 + 0.5;
        let q = [
            a[0] + (b[0] - a[0]) * tt + fz(&mut s, elen),
            a[1] + (b[1] - a[1]) * tt + fz(&mut s, elen),
            a[2] + (b[2] - a[2]) * tt + fz(&mut s, elen),
        ];
        cases.push(([a[0], a[1], a[2], b[0], b[1], b[2]], q));
    }

    for _ in 0..400 {
        let a = [fz(&mut s, 30.0), fz(&mut s, 30.0), fz(&mut s, 30.0)];
        let d = [fz(&mut s, 10.0), fz(&mut s, 10.0), fz(&mut s, 10.0)];
        let b = [a[0] + d[0], a[1] + d[1], a[2] + d[2]];

        let perp = [-d[1], d[0], 0.0];
        let pick = next(&mut s) % 3;
        let base = match pick {
            0 => a,
            1 => b,
            _ => [a[0] + d[0] * 0.5, a[1] + d[1] * 0.5, a[2] + d[2] * 0.5],
        };
        let k = fz(&mut s, 3.0);
        let q = [
            base[0] + perp[0] * k,
            base[1] + perp[1] * k,
            base[2] + perp[2] * k,
        ];
        cases.push(([a[0], a[1], a[2], b[0], b[1], b[2]], q));
    }

    for _ in 0..200 {
        let p = [fz(&mut s, 40.0), fz(&mut s, 40.0), fz(&mut s, 40.0)];
        let q = [
            p[0] + fz(&mut s, 60.0),
            p[1] + fz(&mut s, 60.0),
            p[2] + fz(&mut s, 60.0),
        ];
        cases.push(([p[0], p[1], p[2], p[0], p[1], p[2]], q));
    }

    let mut asserts = 0usize;
    for (ci, (seg, q)) in cases.iter().enumerate() {
        for (i, v) in seg.iter().enumerate() {
            emu.write_u32(seg_va + (i as u64) * 4, v.to_bits());
        }
        for (i, v) in q.iter().enumerate() {
            emu.write_u32(q_va + (i as u64) * 4, v.to_bits());
        }

        emu.write_u32(out_t_va, 0xdead_beef);

        let bin_dist_sq = emu.call_f64(0x7c46d0, seg_va as u32, q_va as u32, &[out_t_va as u32]);
        let bin_t = emu.read_f32(out_t_va);

        let ours = seg_proj(seg, q);

        let ctx = format!("case[{ci}] seg={seg:?} q={q:?}");
        assert_f64(bin_dist_sq, ours.dist_sq, &ctx);
        assert_f32(bin_t, ours.t, &ctx);
        asserts += 2;
    }

    eprintln!(
        "seg_proj: {asserts} bit-exact asserts over {} (segment,query) cases (distSq + out_t each)",
        cases.len()
    );
}
