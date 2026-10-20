//! `HeadingBetweenPoints` (`0x47f220`, `systems/objectlayer/movement.md#heading-between-two-points`)
//! run from the exe.
//!
//! The function returns in `st0`, which its callers consume unrounded, so the test compares the
//! `f64` result through `Emu::call_f64`. `fpatan` routes through the host's maths library in the
//! emulator and in Rust alike, so it compares bit for bit. The inputs include the axis-aligned
//! guards, `2^-22` on either side and NaN; each branch is tallied and required to be hit. Every
//! draw comes off the high bits of an LCG.
//!
//! Host double (`docs/METHOD.md#the-harness`): `geo_47f220` runs `fpatan` at `0x47f2a7`. The
//! emulator computes each in host `double`, so where its result reaches a compared value the host's
//! math stands in for the x87's, and the test shows the code around it.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::heading::heading_between_points as atan2g;

const FUN_47F220: u64 = 0x47f220;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// f32 in `[-range, range)` from the LCG high bits.
fn ff(s: &mut u32, range: f32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    m * 2.0 * range - range
}

/// Compare two `f64`s by bits; both-NaN passes.
fn eqd(mine: f64, bin: f64, ctx: &str) {
    if mine.is_nan() && bin.is_nan() {
        return;
    }
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#018x} ({mine}) vs binary {:#018x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

#[test]
fn geo_47f220() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p1 = emu.alloc(8);
    let p2 = emu.alloc(8);

    struct C {
        p1: [f32; 2],
        p2: [f32; 2],
    }
    let nf = f32::from_bits(0x3f80_0001); // smallest f32 > 1.0 (dx = 1.19e-7 < eps -> vertical)
    let mut cases: Vec<C> = vec![
        // VERTICAL (|dx| < eps): dy>0, dy<0, dy==0 (-> dy>=0 leg), and a tiny-but-nonzero dx leg.
        C {
            p1: [1.0, 1.0],
            p2: [1.0, 5.0],
        }, // dx=0, dy>0  -> 0.5π
        C {
            p1: [1.0, 5.0],
            p2: [1.0, 1.0],
        }, // dx=0, dy<0  -> 1.5π
        C {
            p1: [2.0, 3.0],
            p2: [2.0, 3.0],
        }, // dx=0, dy=0  -> 0.5π
        C {
            p1: [1.0, 0.0],
            p2: [nf, 9.0],
        }, // |dx|=1.19e-7<eps, dy>0 -> 0.5π
        C {
            p1: [1.0, 9.0],
            p2: [nf, 0.0],
        }, // tiny dx, dy<0 -> 1.5π
        // HORIZONTAL (|dy| < eps): dx>0, dx<0, and the tiny-dy leg.
        C {
            p1: [1.0, 2.0],
            p2: [5.0, 2.0],
        }, // dy=0, p2.x>p1.x -> 0.0
        C {
            p1: [5.0, 2.0],
            p2: [1.0, 2.0],
        }, // dy=0, p2.x<p1.x -> π
        C {
            p1: [0.0, 1.0],
            p2: [9.0, nf],
        }, // |dy| tiny, p2.x>p1.x -> 0.0
        C {
            p1: [9.0, 1.0],
            p2: [0.0, nf],
        }, // |dy| tiny, p2.x<p1.x -> π
        // general atan2 quadrant probes.
        C {
            p1: [0.0, 0.0],
            p2: [3.0, 4.0],
        },
        C {
            p1: [0.0, 0.0],
            p2: [-3.0, 4.0],
        },
        C {
            p1: [0.0, 0.0],
            p2: [-3.0, -4.0],
        },
        C {
            p1: [0.0, 0.0],
            p2: [3.0, -4.0],
        },
        // NaN routing (both-NaN passes).
        C {
            p1: [0.0, 0.0],
            p2: [f32::NAN, 1.0],
        },
        C {
            p1: [0.0, 0.0],
            p2: [1.0, f32::NAN],
        },
    ];
    let mut s = 0x47f2_0020u32;
    for _ in 0..6000 {
        // Occasionally force an axis-aligned delta so the guard branches keep firing under fuzz.
        let base = [ff(&mut s, 500.0), ff(&mut s, 500.0)];
        let mode = (next(&mut s) >> 8) % 4;
        let p2 = match mode {
            0 => [base[0], ff(&mut s, 500.0)], // dx exactly 0 -> vertical
            1 => [ff(&mut s, 500.0), base[1]], // dy exactly 0 -> horizontal
            _ => [ff(&mut s, 500.0), ff(&mut s, 500.0)], // general
        };
        cases.push(C { p1: base, p2 });
    }

    // [vert_dyneg, vert_dynonneg, horiz_dxneg, horiz_dxpos, general_atan2]
    let mut tally = [0u32; 5];
    let mut checked = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(p1, c.p1[0].to_bits());
        emu.write_u32(p1 + 4, c.p1[1].to_bits());
        emu.write_u32(p2, c.p2[0].to_bits());
        emu.write_u32(p2 + 4, c.p2[1].to_bits());
        let bin = emu.call_f64(FUN_47F220, p1 as u32, p2 as u32, &[]);
        let mine = atan2g(c.p1, c.p2);
        eqd(
            mine,
            bin,
            &format!("geo_47f220 case {i} p1={:?} p2={:?}", c.p1, c.p2),
        );

        // classify (mirrors the transcription's branch structure) for coverage.
        let dx = (f64::from(c.p2[0]) - f64::from(c.p1[0])) as f32;
        let dy = f64::from(c.p2[1]) - f64::from(c.p1[1]);
        let eps = f64::from(f32::from_bits(0x3480_0000));
        if f64::from(dx).abs() < eps {
            if dy < 0.0 {
                tally[0] += 1;
            } else {
                tally[1] += 1;
            }
        } else if dy.abs() >= eps || dy.is_nan() {
            tally[4] += 1;
        } else if f64::from(c.p2[0]) < f64::from(c.p1[0]) {
            tally[2] += 1;
        } else {
            tally[3] += 1;
        }
        checked += 1;
    }
    eprintln!(
        "geo_47f220: {checked} bit-exact; [vert dy<0={} dy>=0={}  horiz dx<0={} dx>=0={}  atan2={}]",
        tally[0], tally[1], tally[2], tally[3], tally[4]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_47f220 branch bucket {i} never hit");
    }
}
