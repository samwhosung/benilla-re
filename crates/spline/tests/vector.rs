//! `Vector2LengthSquared` (`0x454980`), `Vector3LengthSquared` (`0x4549f0`) and `DivideVector3`
//! (`0x4549c0`), each run whole. The squared lengths come back in `st0` at 53 bits; the division
//! changes the vector in place. The inputs are every pair from the edge set and a fuzz of random
//! non-NaN bit patterns. A NaN result matches any NaN.
#![allow(clippy::float_cmp)]

mod common;

use benilla_re_spline::vector::{vec2_len2, vec3_len2, vec_div_scalar};
use common::*;

/// `0x454980` C3Vector::Length2DSq (`__fastcall` ecx=vec, f32 x@0/y@4 → f64 in st0).
#[test]
fn vec2_len2_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p = emu.alloc(0x20);
    let run = |emu: &mut Emu, x: f32, y: f32| {
        emu.write_u32(p, x.to_bits());
        emu.write_u32(p + 4, y.to_bits());
        let got = emu.call_f64(0x0045_4980, p as u32, 0, &[]);
        eqf64(got, vec2_len2(x, y), &format!("vec2_len2({x},{y})"));
    };
    for &x in EDGES {
        for &y in EDGES {
            run(&mut emu, x, y);
        }
    }
    let mut s = 0x1234_5678u32;
    for _ in 0..4000 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s));
    }
}

/// `0x4549f0` C3Vector::LengthSq (`__fastcall` ecx=vec, f32 x/y/z → f64 in st0; order `(x²+y²)+z²`).
#[test]
fn vec3_len2_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p = emu.alloc(0x20);
    let run = |emu: &mut Emu, x: f32, y: f32, z: f32| {
        emu.write_u32(p, x.to_bits());
        emu.write_u32(p + 4, y.to_bits());
        emu.write_u32(p + 8, z.to_bits());
        let got = emu.call_f64(0x0045_49f0, p as u32, 0, &[]);
        eqf64(got, vec3_len2(x, y, z), &format!("vec3_len2({x},{y},{z})"));
    };
    for &x in EDGES {
        for &y in EDGES {
            run(&mut emu, x, y, 1.5);
        }
    }
    let mut s = 0x2222_3333u32;
    for _ in 0..4000 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), fuzz(&mut s));
    }
}

/// `0x4549c0` C3Vector::operator/=(scalar) (`__fastcall` ecx=vec in/out, [ebp+8]=s; `recip=1/s`, f32 store).
#[test]
fn vec_div_scalar_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p = emu.alloc(0x20);
    let run = |emu: &mut Emu, v: [f32; 3], s: f32| {
        for (i, &c) in v.iter().enumerate() {
            emu.write_u32(p + i as u64 * 4, c.to_bits());
        }
        emu.call(0x0045_49c0, p as u32, 0, &[s.to_bits()]);
        let got = [emu.read_f32(p), emu.read_f32(p + 4), emu.read_f32(p + 8)];
        let want = vec_div_scalar(v, s);
        for i in 0..3 {
            eqf32(got[i], want[i], &format!("vec_div_scalar({v:?},{s})[{i}]"));
        }
    };
    for &c in EDGES {
        for &s in EDGES {
            run(&mut emu, [c, c * 0.5, -c], s);
        }
    }
    let mut st = 0x9999_aaaau32;
    for _ in 0..4000 {
        run(
            &mut emu,
            [fuzz(&mut st), fuzz(&mut st), fuzz(&mut st)],
            fuzz(&mut st),
        );
    }
}
