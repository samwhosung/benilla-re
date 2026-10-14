//! `0x4ead90` (a `MinimapFrame.cpp` function) run from the exe: twice the distance of four values
//! from an anchor pair read from the table at `0xbc7810`.
//!
//! The function writes four `f32` through a caller pointer (`fstp dword`); the test seeds the table
//! entry, reads them back and compares their bits. A pair of NaNs compares equal, but a NaN on one
//! side only fails. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::minimap_geometry::scale_about_anchor as scale_anchor;

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

/// The global anchor table base (`0xbc7810`, stride `0x18`): A at +0, B at +4.
const TABLE: u64 = 0x00bc_7810;

const STRIDE: u64 = 0x18;

#[test]
fn geo_4ead90() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out_va = emu.alloc(0x10);

    // Populate a few table entries (index 0..4) with anchor (A,B) pairs — incl. a large-magnitude
    // anchor (index 3) chosen so (param - anchor) overflows f32 precision: the asymmetry probe.
    let table: [(f32, f32); 5] = [
        (0.0, 0.0),
        (100.0, 200.0),
        (-12.5, 37.25),
        (8_388_608.0, 8_388_608.0), // 2^23: diffs with a fractional param need 25 bits -> f32-round
        (1.0, -1.0),
    ];
    for (idx, &(a, b)) in table.iter().enumerate() {
        let base = TABLE + (idx as u64) * STRIDE;
        emu.write_u32(base, a.to_bits());
        emu.write_u32(base + 4, b.to_bits());
    }

    // (index, [p0,p1,p2,p3]).
    let mut cases: Vec<(u32, [f32; 4])> = vec![
        (0, [1.0, 2.0, 3.0, 4.0]),
        (1, [150.0, 250.0, 50.0, 0.0]),
        (2, [-12.5, 37.25, 0.0, 100.0]),
        // f32-round site probe (index 3): comp0/comp3 take param against a 2^23 anchor, so the diff
        // loses bits when rounded to f32 (round_f32(diff) != diff_f64) — exercising the comp0/comp3
        // `fstp [m32]` site non-trivially. (The *output* still matches the comp1/comp2 path: ×2
        // commutes with round-to-nearest-even — see the f32-round-live tally below.)
        (3, [1.0e7, 0.5, 0.5, 1.0e7]),
    ];
    let mut s = 0x4ead_0090u32;
    for _ in 0..6000 {
        let idx = (next(&mut s) >> 8) % 5;
        // params: mix wide magnitudes (to provoke f32-non-representable diffs) and full entropy.
        let p = [fval(&mut s), fval(&mut s), fval(&mut s), fval(&mut s)];
        cases.push((idx, p));
    }

    // `f32round_live`: cases where the comp0/comp3 early f32-round of the diff is non-trivial
    //   (round_f32(diff) != diff_f64) — proves the `fstp [m32]` sites are genuinely exercised.
    // `output_div`: cases where the f32-round-diff model (mine) and the would-be all-f64 model
    //   produce a DIFFERENT stored f32 — provably 0 (×2 commutes with round-to-nearest-even), the
    //   documented finding that the instruction-level asymmetry is not output-observable.
    let mut f32round_live = 0u32;
    let mut output_div = 0u32;
    for (i, &(idx, p)) in cases.iter().enumerate() {
        let (a, b) = table[idx as usize];
        emu.call(
            0x4ead90,
            0,
            0,
            &[
                out_va as u32,
                idx,
                p[0].to_bits(),
                p[1].to_bits(),
                p[2].to_bits(),
                p[3].to_bits(),
            ],
        );
        let bin = [
            emu.read_f32(out_va),
            emu.read_f32(out_va + 4),
            emu.read_f32(out_va + 8),
            emu.read_f32(out_va + 0xc),
        ];
        let mine = scale_anchor(a, b, p);
        for k in 0..4 {
            assert_f32(mine[k], bin[k], &format!("geo_4ead90 case {i} out[{k}]"));
        }

        // comp0/comp3 diffs: is the early f32-round non-trivial, and does it change the output?
        let d0 = f64::from(p[0]) - f64::from(b);
        let d3 = f64::from(p[3]) - f64::from(a);
        if d0.is_finite() && ((d0 as f32 as f64) != d0 || (d3 as f32 as f64) != d3) {
            f32round_live += 1;
        }
        let naive0 = (2.0 * d0) as f32; // comp0 with the diff left in f64 (no early f32-round)
        let naive3 = (2.0 * d3) as f32;
        let cmp = |x: f32, y: f32| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan());
        if !cmp(naive0, mine[0]) || !cmp(naive3, mine[3]) {
            output_div += 1;
        }
    }
    eprintln!(
        "geo_4ead90: {} bit-exact (comp0/3 round diff->f32, comp1/2 full-precision); \
         f32-round site exercised {f32round_live}x; output-divergence vs all-f64 model: {output_div} \
         (asymmetry is instruction-faithful but NOT output-observable — ×2 commutes with round)",
        cases.len()
    );
    assert!(
        f32round_live > 0,
        "geo_4ead90: the comp0/comp3 f32-round site was never exercised non-trivially"
    );
    // The asymmetry is provably not output-observable for f32 inputs (the documented finding):
    // mine (f32-round-diff) and the all-f64 model must agree bit-for-bit on every case.
    assert_eq!(
        output_div, 0,
        "geo_4ead90: unexpected output divergence — the ×2-commutes-with-round invariant broke"
    );
}
