//! `0x7c2040` run whole against `sphere_aabb_classify` in every mode, with `n` past 3 for the
//! default. The answer is `al`; the rest of `eax` holds what the status word left there.

use benilla_re_cmath::sphere_box::sphere_aabb_classify;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn fz(s: &mut u32, scale: f32) -> f32 {
    (((next(s) >> 9) as f32 / (1u32 << 23) as f32) * 2.0 - 1.0) * scale
}

#[test]
fn sphere_aabb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let box_va = emu.alloc(0x40);
    let sph_va = emu.alloc(0x40);

    let unit_box = [-1.0f32, -1.0, -1.0, 1.0, 1.0, 1.0];
    let mut cases: Vec<([f32; 6], [f32; 4])> = vec![
        (unit_box, [0.0, 0.0, 0.0, 0.1]),
        (unit_box, [0.0, 0.0, 0.0, 1.0]),
        (unit_box, [0.0, 0.0, 0.0, 1.5]),
        (unit_box, [0.0, 0.0, 0.0, 5.0]),
        (unit_box, [1.5, 0.0, 0.0, 0.4]),
        (unit_box, [1.5, 0.0, 0.0, 0.5]),
        (unit_box, [1.5, 0.0, 0.0, 0.6]),
        (unit_box, [2.0, 0.0, 0.0, 0.9]),
        (unit_box, [2.0, 0.0, 0.0, 1.0]),
        (unit_box, [2.0, 0.0, 0.0, 1.1]),
        (unit_box, [-1.5, 0.0, 0.0, 0.5]),
        (unit_box, [-2.0, 0.0, 0.0, 1.5]),
        (unit_box, [0.0, 1.5, 0.0, 0.5]),
        (unit_box, [0.0, -1.5, 0.0, 0.5]),
        (unit_box, [0.0, 2.0, 0.0, 1.0]),
        (unit_box, [2.0, 2.0, 0.0, 1.0]),
        (unit_box, [2.0, 2.0, 0.0, 1.4]),
        (unit_box, [2.0, 2.0, 0.0, 1.5]),
        (unit_box, [-2.0, 2.0, 0.0, 1.5]),
        (unit_box, [-2.0, -2.0, 0.0, 1.5]),
        (unit_box, [1.0, 0.0, 0.0, 0.5]),
        (unit_box, [-1.0, 0.0, 0.0, 0.5]),
        (unit_box, [1.0, 1.0, 0.0, 0.5]),
        (unit_box, [0.0, 0.0, 0.0, 0.0]),
        (unit_box, [1.5, 0.0, 0.0, 0.0]),
        (unit_box, [1.0, 0.0, 0.0, 0.0]),
        ([1.0, 1.0, 1.0, -1.0, -1.0, -1.0], [0.0, 0.0, 0.0, 0.5]),
        ([1.0, 1.0, 1.0, -1.0, -1.0, -1.0], [0.0, 0.0, 0.0, 2.0]),
        ([1.0, 1.0, 1.0, -1.0, -1.0, -1.0], [5.0, 5.0, 0.0, 0.5]),
        ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.5]),
        ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.5]),
        ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 1.5]),
        ([2.0, 3.0, 0.0, 2.0, 3.0, 0.0], [2.0, 3.0, 0.0, 0.0]),
        ([-3.0, -1.0, 0.0, 5.0, 2.0, 1.0], [4.0, 1.0, 0.0, 0.7]),
        ([-3.0, -1.0, 0.0, 5.0, 2.0, 1.0], [6.0, 3.0, 0.0, 2.5]),
        (unit_box, [-0.0, -0.0, 0.0, 0.5]),
        (unit_box, [f32::NAN, 0.0, 0.0, 0.5]),
        (unit_box, [0.0, f32::NAN, 0.0, 0.5]),
        (unit_box, [0.0, 0.0, 0.0, f32::NAN]),
        ([f32::NAN, -1.0, -1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 0.5]),
        ([-1.0, -1.0, -1.0, f32::NAN, 1.0, 1.0], [0.0, 0.0, 0.0, 0.5]),
        (unit_box, [f32::INFINITY, 0.0, 0.0, 0.5]),
        (unit_box, [0.0, 0.0, 0.0, f32::INFINITY]),
        (unit_box, [f32::NEG_INFINITY, 0.0, 0.0, 0.5]),
    ];

    let mut s = 0x51ed_270bu32;
    for _ in 0..2000 {
        let bx0 = fz(&mut s, 4.0);
        let by0 = fz(&mut s, 4.0);
        let ex = fz(&mut s, 3.0);
        let ey = fz(&mut s, 3.0);
        let boxf = [bx0, by0, -1.0, bx0 + ex, by0 + ey, 1.0];

        let cx = bx0 + ex * 0.5 + fz(&mut s, 4.0);
        let cy = by0 + ey * 0.5 + fz(&mut s, 4.0);
        let r = (fz(&mut s, 3.0)).abs();
        cases.push((boxf, [cx, cy, 0.0, r]));
    }

    for _ in 0..400 {
        let h = (fz(&mut s, 3.0)).abs() + 0.1;
        let boxf = [-h, -h, -1.0, h, h, 1.0];

        let pick = next(&mut s) % 4;
        let (cx, cy) = match pick {
            0 => (h, fz(&mut s, h)),
            1 => (-h, fz(&mut s, h)),
            2 => (fz(&mut s, h), h),
            _ => (h, h),
        };
        let r = (fz(&mut s, 2.0)).abs();
        cases.push((boxf, [cx, cy, 0.0, r]));
    }

    let mut asserts = 0usize;
    for (ci, (boxf, sph)) in cases.iter().enumerate() {
        for (i, v) in boxf.iter().enumerate() {
            emu.write_u32(box_va + (i as u64) * 4, v.to_bits());
        }
        for (i, v) in sph.iter().enumerate() {
            emu.write_u32(sph_va + (i as u64) * 4, v.to_bits());
        }
        for n in 0u32..6 {
            let eax = emu.call(0x7c2040, box_va as u32, sph_va as u32, &[n]);
            let got = eax & 0xff;
            let ours = sphere_aabb_classify(boxf, sph, n);
            assert_eq!(
                got, ours,
                "0x7c2040 case[{ci}] n={n} box={boxf:?} sph={sph:?}: binary_al={got:#x} ours={ours:#x}"
            );
            asserts += 1;
        }
    }
    eprintln!(
        "sphere_aabb: {asserts} bit-exact asserts over {} (box,sphere) cases × 6 modes",
        cases.len()
    );
}
