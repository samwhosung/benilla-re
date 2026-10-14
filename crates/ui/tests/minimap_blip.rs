//! `ProjectBlip` (`0x4eaa30`, `systems/ui/minimapblips.md#the-north-up-disc`) run from the exe.
//!
//! The function returns `void` and writes two `f32` through the out pointer in `ecx` (`ret 0x20`);
//! the test reads both slots back and compares their bits. Inputs are finite, with a nonzero finite
//! depth. A probe checks that all three narrowing sites matter: leaving out any one of them gives a
//! different result. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::minimap_geometry::project_blip as geo;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A finite float in `[-range, range)` from the LCG's high bits.
fn ff(s: &mut u32, range: f32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
    m * 2.0 * range - range
}

/// A non-zero, finite `depth` spanning moderate / tiny-magnitude (large `inv`) / very-large
/// (tiny `inv`), both signs.
fn depthv(s: &mut u32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    let mag = match (next(s) >> 8) % 4 {
        0 => m * 1000.0 + 0.5,  // moderate
        1 => m * 0.02 + 1e-4,   // small -> large inv
        2 => m * 1.0e6 + 1.0,   // very large -> tiny inv
        _ => m * 200.0 + 0.125, // moderate
    };
    if next(s) & 0x800 != 0 {
        -mag
    } else {
        mag
    }
}

/// Compare two `f32`s by bits; both-NaN (routing matched, payload unspeced) passes.
fn assert_bits(mine: f32, bin: f32, ctx: &str) {
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

#[test]
fn geo_4eaa30() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out = emu.alloc(8);

    // (a8, ac, depth, a18, a1c, size, scale). a10 (the unused [ebp+0x10] slot) is written 0.
    struct C {
        a8: f32,
        ac: f32,
        depth: f32,
        a18: f32,
        a1c: f32,
        size: f32,
        scale: f32,
    }
    // The load-bearing probe (case 0): values chosen so f=size·scale·0.5 and both products carry
    // mantissa bits past f32, making each of the three round sites observable. Asserted below.
    let probe = C {
        a8: 1.3,
        ac: 2.9,
        depth: 3.0,
        a18: 200.7,
        a1c: 7.1,
        size: 100.1,
        scale: 0.7,
    };
    let mut cases: Vec<C> = vec![
        probe,
        // small/identity-ish sanity probes.
        C {
            a8: 0.0,
            ac: 0.0,
            depth: 1.0,
            a18: 1.0,
            a1c: 1.0,
            size: 2.0,
            scale: 1.0,
        },
        C {
            a8: -5.0,
            ac: 4.0,
            depth: -2.0,
            a18: 5.0,
            a1c: -4.0,
            size: 10.0,
            scale: 0.5,
        },
        // tiny depth -> large inv.
        C {
            a8: 0.5,
            ac: 0.25,
            depth: 1e-4,
            a18: 0.75,
            a1c: 0.5,
            size: 1.0,
            scale: 1.0,
        },
        // huge depth -> tiny inv (offsets collapse toward f).
        C {
            a8: 10.0,
            ac: 20.0,
            depth: 1e6,
            a18: 30.0,
            a1c: 40.0,
            size: 64.0,
            scale: 1.5,
        },
    ];
    let mut s = 0x4eaa_0030u32;
    for _ in 0..5000 {
        cases.push(C {
            a8: ff(&mut s, 300.0),
            ac: ff(&mut s, 300.0),
            depth: depthv(&mut s),
            a18: ff(&mut s, 300.0),
            a1c: ff(&mut s, 300.0),
            size: ff(&mut s, 200.0),
            scale: ff(&mut s, 4.0),
        });
    }

    let mut tally = [0u32; 4]; // [depth>0, depth<0, |inv|>=1 (small depth), |inv|<1 (large depth)]
    let mut checked = 0u32;
    for (i, c) in cases.iter().enumerate() {
        // stack: [a8, ac, a10(unused=0), depth, a18, a1c, size, scale].
        let args = [
            c.a8.to_bits(),
            c.ac.to_bits(),
            0u32,
            c.depth.to_bits(),
            c.a18.to_bits(),
            c.a1c.to_bits(),
            c.size.to_bits(),
            c.scale.to_bits(),
        ];
        emu.write_u32(out, 0);
        emu.write_u32(out + 4, 0);
        emu.call(0x4eaa30, out as u32, 0, &args);
        let bin = [emu.read_f32(out), emu.read_f32(out + 4)];
        let mine = geo(c.a8, c.ac, c.depth, c.a18, c.a1c, c.size, c.scale);
        assert_bits(mine[0], bin[0], &format!("geo_4eaa30 case {i} out[0]"));
        assert_bits(mine[1], bin[1], &format!("geo_4eaa30 case {i} out[1]"));

        tally[if c.depth > 0.0 { 0 } else { 1 }] += 1;
        if (1.0 / f64::from(c.depth)).abs() >= 1.0 {
            tally[2] += 1;
        } else {
            tally[3] += 1;
        }
        checked += 1;
    }

    // Load-bearing check: the three intermediate f32-round sites each change the probe's result.
    // Variants skip exactly one round; each must differ from the (bit-exact) transcription.
    let p = &cases[0];
    let dx = f64::from(p.a18) - f64::from(p.a8);
    let dy = f64::from(p.a1c) - f64::from(p.ac);
    let f_full = f64::from(p.size) * f64::from(p.scale) * 0.5;
    let f_f32 = f64::from(f_full as f32);
    let inv = 1.0 / f64::from(p.depth);
    let correct = geo(p.a8, p.ac, p.depth, p.a18, p.a1c, p.size, p.scale);
    // skip site 1: use full-precision f (not f_f32) in the final add/sub.
    let s1 = {
        let fdx = f64::from((f_full * dx) as f32);
        let fdx_inv = f64::from((fdx * inv) as f32);
        [
            (f_full - (f_full * dy) * inv) as f32,
            (f_full + fdx_inv) as f32,
        ]
    };
    // skip site 2: do not round f·dx.
    let s2 = {
        let fdx = f_full * dx; // unrounded
        let fdx_inv = f64::from((fdx * inv) as f32);
        [
            (f_f32 - (f_full * dy) * inv) as f32,
            (f_f32 + fdx_inv) as f32,
        ]
    };
    // skip site 3: do not round (f·dx)·inv.
    let s3 = {
        let fdx = f64::from((f_full * dx) as f32);
        let fdx_inv = fdx * inv; // unrounded
        [
            (f_f32 - (f_full * dy) * inv) as f32,
            (f_f32 + fdx_inv) as f32,
        ]
    };
    assert!(
        s1 != correct,
        "site 1 (f spill->f32) not load-bearing on probe: {s1:?} == {correct:?}"
    );
    assert!(
        s2 != correct,
        "site 2 (f·dx->f32) not load-bearing on probe: {s2:?} == {correct:?}"
    );
    assert!(
        s3 != correct,
        "site 3 ((f·dx)·inv->f32) not load-bearing on probe: {s3:?} == {correct:?}"
    );

    eprintln!(
        "geo_4eaa30: {checked} bit-exact; [depth+={} depth-={} |inv|>=1={} |inv|<1={}]; \
         3/3 f32-round sites confirmed load-bearing on probe",
        tally[0], tally[1], tally[2], tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_4eaa30 coverage bucket {i} never hit");
    }
}
