//! `NormalizeTaxiPoint` (`0x4dc890`, `systems/ui/taxi.md#map-art-and-positions`) run from the exe.
//!
//! The function writes three `f32` through a caller pointer (`fstp dword`); the test reads them back
//! and compares their bits. A pair of NaNs compares equal, as the x87's quiet NaN payload is not
//! part of the specification, but a NaN on one side only still fails. Every draw comes off the high
//! bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::taxi_map::normalize_taxi_point as inv_lerp;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// Compare a transcribed `f32` against the binary's stored `f32`: bit-exact, except both-NaN passes.
fn assert_f32(mine: f32, bin: f32, ctx: &str) {
    if mine.is_nan() && bin.is_nan() {
        return;
    }
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#010x} ({mine}) vs binary {:#010x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

/// A finite float spanning a wide magnitude/sign range, occasionally with many mantissa bits.
fn fval(s: &mut u32) -> f32 {
    let r = next(s);
    match (r >> 8) % 8 {
        0 => 0.0,
        1 => -0.0,
        2 => {
            // large magnitude (exercises Sterbenz-violating diffs -> f32-round matters).
            let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
            (m * 2.0 - 1.0) * 1.6e7
        }
        3 => {
            // tiny magnitude.
            let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
            (m * 2.0 - 1.0) * 1e-3
        }
        _ => {
            // mid-range with full mantissa entropy.
            let bits = (next(s) & 0x007f_ffff) | ((next(s) & 0xff) << 23);
            let v = f32::from_bits(bits);
            if v.is_finite() {
                v
            } else {
                1.0
            }
        }
    }
}

#[test]
fn geo_4dc890() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out_va = emu.alloc(0x10);
    let coeffs_va = emu.alloc(0x10);
    let point_va = emu.alloc(0x10);

    // explicit probes: clean interior, zero denominators (->inf/nan), negative spans.
    // coeffs = [x0, y0, x1, y1], point = [px, py].
    let mut cases: Vec<([f32; 4], [f32; 2])> = vec![
        ([0.0, 0.0, 100.0, 50.0], [25.0, 10.0]),   // interior
        ([10.0, 20.0, 10.0, 50.0], [25.0, 30.0]),  // x1==x0 -> out[0] denom 0
        ([10.0, 20.0, 100.0, 20.0], [25.0, 30.0]), // y1==y0 -> out[1] denom 0
        ([10.0, 20.0, 10.0, 20.0], [10.0, 20.0]),  // both denom 0, both numers 0 -> NaN
        ([100.0, 50.0, 0.0, 0.0], [25.0, 10.0]),   // negative spans
        ([-5.0, -8.0, 5.0, 8.0], [0.0, 0.0]),      // straddling zero
    ];
    let mut s = 0x4dc8_0090u32;
    for _ in 0..5000 {
        let mut c = [fval(&mut s), fval(&mut s), fval(&mut s), fval(&mut s)];
        // ~1/6 of the time collapse a denominator to zero exactly.
        match (next(&mut s) >> 8) % 6 {
            0 => c[2] = c[0], // x1 = x0
            1 => c[3] = c[1], // y1 = y0
            _ => {}
        }
        let p = [fval(&mut s), fval(&mut s)];
        cases.push((c, p));
    }

    let mut tally = [0u32; 3]; // [finite, has_inf, has_nan]
    for (i, &(c, p)) in cases.iter().enumerate() {
        for (k, v) in c.iter().enumerate() {
            emu.write_u32(coeffs_va + (k as u64) * 4, v.to_bits());
        }
        for (k, v) in p.iter().enumerate() {
            emu.write_u32(point_va + (k as u64) * 4, v.to_bits());
        }
        emu.write_u32(out_va + 8, 0xdead_beef); // poison: confirm the function writes out[2]=0.
        emu.call(
            0x4dc890,
            out_va as u32,
            coeffs_va as u32,
            &[point_va as u32],
        );
        let bin = [
            emu.read_f32(out_va),
            emu.read_f32(out_va + 4),
            emu.read_f32(out_va + 8),
        ];
        let mine = inv_lerp(c, p);
        for k in 0..3 {
            assert_f32(mine[k], bin[k], &format!("geo_4dc890 case {i} out[{k}]"));
        }
        if mine[0].is_nan() || mine[1].is_nan() {
            tally[2] += 1;
        } else if mine[0].is_infinite() || mine[1].is_infinite() {
            tally[1] += 1;
        } else {
            tally[0] += 1;
        }
    }
    eprintln!(
        "geo_4dc890: {} bit-exact (out[0],out[1] f32 sites + out[2]=0); [finite={} inf={} nan={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_4dc890 leg {i} never hit");
    }
}
