//! `BoxContainsTriangle` (`0x6b8b70`) run whole: points inside, outside one side, mixed, and random
//! boxes and points, some of them any bits.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_models::box_points::aabb_points3_separated;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

#[test]
fn aabb_points3_separated_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let aabb = e.alloc(0x20);
    let p0 = e.alloc(0x10);
    let p1 = e.alloc(0x10);
    let p2 = e.alloc(0x10);
    let mut s = 0x6b8b_0000u32;
    let (mut sep, mut inside) = (0u32, 0u32);
    let mut checked = 0u32;

    let mut case = |e: &mut Emu, ab: [f32; 6], pts: [[f32; 3]; 3], ctx: &str| {
        for i in 0..6u64 {
            e.write_u32(aabb + i * 4, ab[i as usize].to_bits());
        }
        for (slot, p) in [p0, p1, p2].iter().zip(pts.iter()) {
            for k in 0..3u64 {
                e.write_u32(slot + k * 4, p[k as usize].to_bits());
            }
        }
        // ecx = the box, edx = p0, then p1 and p2 on the stack; al is the result
        let al = e.call(0x6b8b70, aabb as u32, p0 as u32, &[p1 as u32, p2 as u32]) & 0xff;
        let ours = aabb_points3_separated(&ab, [&pts[0], &pts[1], &pts[2]]);
        assert_eq!(al != 0, ours, "{ctx}: binary AL={al} ours={ours}");
        if ours {
            sep += 1;
        } else {
            inside += 1;
        }
    };

    // A box from 0 to 10: points inside, all above the maximum x, all below the minimum z, mixed.
    let box1 = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0];
    case(
        &mut e,
        box1,
        [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
        "all inside (not separated)",
    );
    case(
        &mut e,
        box1,
        [[20.0, 1.0, 1.0], [30.0, 2.0, 2.0], [40.0, 3.0, 3.0]],
        "all above max.x (separated)",
    );
    case(
        &mut e,
        box1,
        [[1.0, 1.0, -5.0], [2.0, 2.0, -6.0], [3.0, 3.0, -7.0]],
        "all below min.z (separated)",
    );
    case(
        &mut e,
        box1,
        [[1.0, 1.0, 1.0], [20.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
        "mixed (one above) — not all → not separated",
    );

    for i in 0..4000u32 {
        let lo = [
            frand(&mut s, -50.0, 0.0),
            frand(&mut s, -50.0, 0.0),
            frand(&mut s, -50.0, 0.0),
        ];
        let hi = [
            frand(&mut s, 0.0, 50.0),
            frand(&mut s, 0.0, 50.0),
            frand(&mut s, 0.0, 50.0),
        ];
        let ab = [lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]];
        // a third of the time the points cluster above the box; some are any bits
        let pts: [[f32; 3]; 3] = std::array::from_fn(|_| {
            if i % 3 == 0 {
                [
                    frand(&mut s, 60.0, 200.0),
                    frand(&mut s, 60.0, 200.0),
                    frand(&mut s, 60.0, 200.0),
                ]
            } else if i % 7 == 0 {
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
            } else {
                [
                    frand(&mut s, -80.0, 80.0),
                    frand(&mut s, -80.0, 80.0),
                    frand(&mut s, -80.0, 80.0),
                ]
            }
        });
        case(&mut e, ab, pts, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(
        sep > 0 && inside > 0,
        "both dispositions exercised (separated={sep} inside={inside})"
    );
    eprintln!(
        "aabb_points3_separated: {checked} cases bit-exact vs WoW.exe @ 0x6b8b70 (sep={sep} inside={inside})"
    );
}
