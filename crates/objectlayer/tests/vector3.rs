//! `ScaleVector3` (`0x5f8cf0`) and `AddVector3` (`0x621160`) against the exe, run whole on edge
//! values (zeros of both signs, NaN, an infinity, tiny and huge magnitudes) and random vectors in
//! about ±1000. `ecx` is the output, `edx` the first operand and the stack the second.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::vector3::{vec_add, vec_scale};
use common::{assert_f32, next};

/// A draw in about [-1e3, 1e3).
fn fz(s: &mut u32) -> f32 {
    (((next(s) >> 9) as f32 / (1 << 23) as f32) * 2.0 - 1.0) * 1.0e3
}

#[test]
fn vector3_scale_add_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out_va = emu.alloc(0x20);
    let a_va = emu.alloc(0x20);
    let b_va = emu.alloc(0x20);
    let mut s = 0x51c0_a73fu32;

    let edge_scalars = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -2.0,
        1.0e-20,
        1.0e20,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let edge_vecs: [[f32; 3]; 7] = [
        [0.0, 0.0, 0.0],
        [1.0, -2.0, 3.0],
        [1.0e-20, 1.0e20, -1.0e-20],
        [-0.0, 0.0, -1.0],
        [2.5, -3.75, 0.125],
        [f32::INFINITY, f32::NEG_INFINITY, f32::NAN],
        [-0.0, -0.0, -0.0],
    ];

    let scale = |emu: &mut Emu, v: &[f32; 3], sc: f32| {
        for (i, c) in v.iter().enumerate() {
            emu.write_u32(a_va + (i as u64) * 4, c.to_bits());
        }
        emu.call(0x5f8cf0, out_va as u32, a_va as u32, &[sc.to_bits()]);
        for (k, w) in vec_scale(v, sc).iter().enumerate() {
            let got = emu.read_f32(out_va + (k as u64) * 4);
            assert_f32(got, *w, &format!("0x5f8cf0 scale[{k}] s={sc}"));
        }
    };
    for &v in &edge_vecs {
        for &sc in &edge_scalars {
            scale(&mut emu, &v, sc);
        }
    }
    for _ in 0..1500 {
        let v = [fz(&mut s), fz(&mut s), fz(&mut s)];
        scale(&mut emu, &v, fz(&mut s));
    }

    let add = |emu: &mut Emu, a: &[f32; 3], b: &[f32; 3]| {
        for (i, c) in a.iter().enumerate() {
            emu.write_u32(a_va + (i as u64) * 4, c.to_bits());
        }
        for (i, c) in b.iter().enumerate() {
            emu.write_u32(b_va + (i as u64) * 4, c.to_bits());
        }
        emu.call(0x621160, out_va as u32, a_va as u32, &[b_va as u32]);
        for (k, w) in vec_add(a, b).iter().enumerate() {
            let got = emu.read_f32(out_va + (k as u64) * 4);
            assert_f32(got, *w, &format!("0x621160 add[{k}]"));
        }
    };
    for &a in &edge_vecs {
        for &b in &edge_vecs {
            add(&mut emu, &a, &b);
        }
    }
    for _ in 0..1500 {
        let a = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let b = [fz(&mut s), fz(&mut s), fz(&mut s)];
        add(&mut emu, &a, &b);
    }
}
