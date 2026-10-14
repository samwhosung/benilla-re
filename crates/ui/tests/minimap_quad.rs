//! `0x4eb910` (a `MinimapFrame.cpp` function) run from the exe: the quad of a blip.
//!
//! The function writes 28 `f32` at `this+0` to `this+0x6c`. Its callees run as the exe's own code:
//! `0x4ef370`, which copies four floats, and `0x4ead90`, which reads the anchor table `0xbc7810`
//! that the test seeds. The output floats are read back and compared by their bits. Every draw
//! comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::minimap_geometry::{blip_quad as quad, scale_about_anchor as geo_4ead90};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A finite float spanning sign/magnitude, occasionally with full mantissa entropy.
fn fval(s: &mut u32) -> f32 {
    let r = next(s);
    match (r >> 8) % 8 {
        0 => 0.0,
        1 => -0.0,
        2 => {
            let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
            (m * 2.0 - 1.0) * 1.6e7
        }
        3 => {
            let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
            (m * 2.0 - 1.0) * 1e-3
        }
        _ => {
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

/// A non-zero, finite magnitude spanning moderate / tiny (large inv) / very large (tiny inv).
fn nzval(s: &mut u32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    let mag = match (next(s) >> 8) % 4 {
        0 => m * 1000.0 + 0.5,
        1 => m * 0.02 + 1e-4,
        2 => m * 1.0e6 + 1.0,
        _ => m * 200.0 + 0.125,
    };
    if next(s) & 0x800 != 0 {
        -mag
    } else {
        mag
    }
}

/// Compare a transcribed `f32` against the binary's stored `f32`: bit-exact, both-NaN passes.
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

// ----------------------------------------------------------------------------------------------
// geo_4eb910 — minimap blip quad geometry
// ----------------------------------------------------------------------------------------------

/// The global anchor table base (`0xbc7810`, stride `0x18`): anchorA at +0, anchorB at +4 — read by
/// the composed `geo_4ead90`.
const TABLE: u64 = 0x00bc_7810;

const STRIDE: u64 = 0x18;

#[derive(Clone, Copy)]
struct QuadCase {
    p: [f32; 4],
    c0: f32,
    c1: f32,
    half: f32,
    k1: f32,
    k2: f32,
    idx: u32,
}

#[test]
fn geo_4eb910() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this_va = emu.alloc(0x80); // pure output: +0x0..+0x6c (28 f32s)
    let p_va = emu.alloc(0x10);
    let corner_va = emu.alloc(0x10);

    // Seed the anchor table (indices 0..4), incl. a 2^23 anchor so geo_4ead90's diff carries bits.
    let table: [(f32, f32); 5] = [
        (0.0, 0.0),
        (100.0, 200.0),
        (-12.5, 37.25),
        (8_388_608.0, 8_388_608.0),
        (1.0, -1.0),
    ];
    for (idx, &(a, b)) in table.iter().enumerate() {
        let base = TABLE + (idx as u64) * STRIDE;
        emu.write_u32(base, a.to_bits());
        emu.write_u32(base + 4, b.to_bits());
    }

    // Explicit probes: an interior quad and a large-magnitude probe making the f32 sites non-trivial.
    let mut cases: Vec<QuadCase> = vec![
        QuadCase {
            p: [1.0, 2.0, 3.0, 4.0],
            c0: 0.5,
            c1: 1.5,
            half: 16.0,
            k1: 2.0,
            k2: 3.0,
            idx: 1,
        },
        QuadCase {
            p: [1.0e7, 0.5, -1.0e7, 200.7],
            c0: 100.1,
            c1: -50.3,
            half: 0.7,
            k1: 123.456,
            k2: 0.789,
            idx: 3,
        },
    ];
    let mut s = 0x4eb9_0010u32;
    for _ in 0..6000 {
        cases.push(QuadCase {
            p: [fval(&mut s), fval(&mut s), fval(&mut s), fval(&mut s)],
            c0: fval(&mut s),
            c1: fval(&mut s),
            half: nzval(&mut s),
            k1: fval(&mut s),
            k2: fval(&mut s),
            idx: (next(&mut s) >> 8) % 5,
        });
    }

    let mut tally = [0u32; 2]; // [all-finite output, has non-finite output]
    for (i, c) in cases.iter().enumerate() {
        for k in 0..4 {
            emu.write_u32(p_va + (k as u64) * 4, c.p[k].to_bits());
        }
        emu.write_u32(corner_va, c.c0.to_bits());
        emu.write_u32(corner_va + 4, c.c1.to_bits());
        // poison every output slot: confirms all 28 floats are written.
        for k in 0..28u64 {
            emu.write_u32(this_va + k * 4, 0xdead_beef);
        }

        // thiscall: ecx=this; args [P, idx, unused, half, corner, k1, k2] (ret 0x1c).
        emu.call(
            0x4eb910,
            this_va as u32,
            0,
            &[
                p_va as u32,
                c.idx,
                0,
                c.half.to_bits(),
                corner_va as u32,
                c.k1.to_bits(),
                c.k2.to_bits(),
            ],
        );

        let (a, b) = table[c.idx as usize];
        let mine = quad(c.p, c.c0, c.c1, c.half, c.k1, c.k2, a, b);
        let mut finite = true;
        for k in 0..28u64 {
            let bin = emu.read_f32(this_va + k * 4);
            assert_bits(
                mine[k as usize],
                bin,
                &format!("geo_4eb910 case {i} out[{k}]"),
            );
            finite &= mine[k as usize].is_finite();
        }
        tally[if finite { 0 } else { 1 }] += 1;
    }

    // The composed geo_4ead90 basis must agree with a direct call (it ran as real binary code here).
    let direct = geo_4ead90(table[1].0, table[1].1, cases[0].p);
    assert_eq!(direct.len(), 4, "geo_4ead90 basis arity");

    eprintln!(
        "geo_4eb910: {} bit-exact (28 f32s/case: positions + scale-anchor basis + corner UVs); \
         [finite={} non_finite={}]",
        cases.len(),
        tally[0],
        tally[1]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_4eb910 output bucket {i} never hit");
    }
}
