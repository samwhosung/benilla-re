//! The integer square root `0x454a10` and the normalization of a vector of two `0x456280`, each run
//! whole. The integer root runs the edges of its three starting brackets, squares and their
//! neighbours, and random words; the normalization runs every pair from the edge set (the zero
//! vector gives an infinite reciprocal) and a fuzz. A NaN result matches any NaN.
#![allow(clippy::float_cmp)]

mod common;

use benilla_re_spline::scalar::{isqrt, normalize2_f32};
use common::*;

// =================================================================================================
// 0x454a10 — integer Newton/Heron sqrt. `__fastcall` ecx=n → eax. Pure integer.
// =================================================================================================
#[test]
fn isqrt_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let run = |emu: &mut Emu, n: u32| {
        let got = emu.call(0x0045_4a10, n, 0, &[]);
        eqi(got, isqrt(n), &format!("isqrt({n})"));
    };
    // boundary probes: 0, 1, the bracket edges (0xff, 0x100, 0xffff, 0x10000), perfect squares,
    // off-by-one around squares, large.
    for &n in &[
        0u32,
        1,
        2,
        3,
        4,
        8,
        9,
        15,
        16,
        0xff,
        0x100,
        0x101,
        0xfffe,
        0xffff,
        0x1_0000,
        0x1_0001,
        1_000_000,
        0x3fff_ffff,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fffe,
        0xffff_ffff,
    ] {
        run(&mut emu, n);
    }
    // squares & neighbours.
    for k in [3u32, 100, 46340, 65535, 100_000, 1_000_000] {
        let sq = k.wrapping_mul(k);
        for d in [0u32, 1, 2] {
            run(&mut emu, sq.wrapping_sub(d));
            run(&mut emu, sq.wrapping_add(d));
        }
    }
    let mut s = 0xfeed_face_u32;
    for _ in 0..20000 {
        run(&mut emu, lcg(&mut s));
    }
}

/// `0x456280` vec2 normalize (f32). ecx=&x, edx=&y (no stack arg).
#[test]
fn normalize2_f32_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let px = emu.alloc(0x10);
    let py = emu.alloc(0x10);
    let run = |emu: &mut Emu, x: f32, y: f32| {
        emu.write_u32(px, x.to_bits());
        emu.write_u32(py, y.to_bits());
        emu.call(0x0045_6280, px as u32, py as u32, &[]);
        let got = [emu.read_f32(px), emu.read_f32(py)];
        let want = normalize2_f32(x, y);
        for i in 0..2 {
            eqf32(got[i], want[i], &format!("normalize2_f32({x},{y})[{i}]"));
        }
    };
    // boundary probes: zero vector (1/0 → inf), unit vectors.
    run(&mut emu, 0.0, 0.0);
    run(&mut emu, 1.0, 0.0);
    run(&mut emu, 0.0, 1.0);
    run(&mut emu, 3.0, 4.0); // 3-4-5
    for &x in EDGES {
        for &y in EDGES {
            run(&mut emu, x, y);
        }
    }
    let mut s = 0x1357_9bdfu32;
    for _ in 0..4000 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s));
    }
}
