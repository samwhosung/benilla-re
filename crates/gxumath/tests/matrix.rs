//! The projection and view builders against the exe, each run whole: the perspective `0x5c3cc0`,
//! the orthographic `0x5c3d90` and the look-at `0x5c3e70`. Each writes sixteen `f32` through its
//! output pointer, which is read back. The arguments are random and kept inside each builder's
//! validity gate, so the exe never takes its `SErrSetLastError` path. The `fptan` of the perspective
//! is the host's tangent, which the emulator uses too; the half-angle stays under 1.6.
//!
//! Host double (`docs/METHOD.md#the-harness`): `perspective_difftest` runs `fptan` at `0x5c3d4c`.
//! The emulator computes each in host `double`, so where its result reaches a compared value the
//! host's math stands in for the x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gxumath::matrix::{lookat_v1, projection_ortho, projection_perspective};
use common::{assert_arr, fuzz, fuzz_pos};

const PERSPECTIVE: u64 = 0x5c3cc0; // __thiscall: ecx=dst, stack fov,aspect,near,far
const ORTHO: u64 = 0x5c3d90; // __thiscall: ecx=dst, stack l,r,b,t,n,f
const LOOKAT_V1: u64 = 0x5c3e70; // __fastcall: ecx=eye, edx=target, stack up,dst

fn rd16(e: &Emu, va: u64) -> [f32; 16] {
    let mut m = [0f32; 16];
    for (k, slot) in m.iter_mut().enumerate() {
        *slot = e.read_f32(va + 4 * k as u64);
    }
    m
}

fn wr3(e: &mut Emu, va: u64, v: [f32; 3]) {
    e.write_u32(va, v[0].to_bits());
    e.write_u32(va + 4, v[1].to_bits());
    e.write_u32(va + 8, v[2].to_bits());
}

fn len2(v: [f32; 3]) -> f64 {
    f64::from(v[0]) * f64::from(v[0])
        + f64::from(v[1]) * f64::from(v[1])
        + f64::from(v[2]) * f64::from(v[2])
}

#[test]
fn perspective_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let dst = e.alloc(0x40);
    let mut s: u32 = 0x5c3c_0001;
    for i in 0..4000u32 {
        let fov = fuzz_pos(&mut s, 3.1); // (0, 3.1] < π
        let aspect = fuzz_pos(&mut s, 4.0); // (0, 4]
        let n0 = fuzz_pos(&mut s, 500.0);
        let f0 = fuzz_pos(&mut s, 500.0);
        if n0 == f0 {
            continue; // need near < far (strict)
        }
        let (near, far) = if n0 < f0 { (n0, f0) } else { (f0, n0) };

        e.write(dst, &[0u8; 0x40]);
        e.call(
            PERSPECTIVE,
            dst as u32,
            0,
            &[
                fov.to_bits(),
                aspect.to_bits(),
                near.to_bits(),
                far.to_bits(),
            ],
        );
        assert_arr(
            &rd16(&e, dst),
            &projection_perspective(fov, aspect, near, far),
            &format!("perspective[{i}] fov={fov} aspect={aspect} near={near} far={far}"),
        );
    }
}

#[test]
fn ortho_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let dst = e.alloc(0x40);
    let mut s: u32 = 0x5c3d_0001;
    for i in 0..4000u32 {
        let left = fuzz(&mut s, 200.0);
        let right = fuzz(&mut s, 200.0);
        let bottom = fuzz(&mut s, 200.0);
        let top = fuzz(&mut s, 200.0);
        let n0 = fuzz(&mut s, 500.0);
        let f0 = fuzz(&mut s, 500.0);
        // validity gate: l≠r, b≠t, n<f
        if left == right || bottom == top || n0 == f0 {
            continue;
        }
        let (near, far) = if n0 < f0 { (n0, f0) } else { (f0, n0) };

        e.write(dst, &[0u8; 0x40]);
        e.call(
            ORTHO,
            dst as u32,
            0,
            &[
                left.to_bits(),
                right.to_bits(),
                bottom.to_bits(),
                top.to_bits(),
                near.to_bits(),
                far.to_bits(),
            ],
        );
        assert_arr(
            &rd16(&e, dst),
            &projection_ortho(left, right, bottom, top, near, far),
            &format!("ortho[{i}] l={left} r={right} b={bottom} t={top} n={near} f={far}"),
        );
    }
}

/// The look-at driver: the call shape and the validity gate.
fn lookat_check(seed: u32, fn_va: u64, rust: impl Fn([f32; 3], [f32; 3], [f32; 3]) -> [f32; 16]) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let eye_p = e.alloc(0xc);
    let tgt_p = e.alloc(0xc);
    let up_p = e.alloc(0xc);
    let dst = e.alloc(0x40);
    let mut s: u32 = seed;
    for i in 0..4000u32 {
        let eye = [
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
        ];
        let target = [
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
        ];
        let up = [fuzz(&mut s, 10.0), fuzz(&mut s, 10.0), fuzz(&mut s, 10.0)];
        let dir = [
            (f64::from(target[0]) - f64::from(eye[0])) as f32,
            (f64::from(target[1]) - f64::from(eye[1])) as f32,
            (f64::from(target[2]) - f64::from(eye[2])) as f32,
        ];
        // gate: len²(dir), len²(up) ≥ 0.01 — keep a generous margin so the binary never asserts.
        if len2(dir) < 1.0 || len2(up) < 1.0 {
            continue;
        }

        wr3(&mut e, eye_p, eye);
        wr3(&mut e, tgt_p, target);
        wr3(&mut e, up_p, up);
        e.write(dst, &[0u8; 0x40]);
        e.call(
            fn_va,
            eye_p as u32,
            tgt_p as u32,
            &[up_p as u32, dst as u32],
        );
        assert_arr(
            &rd16(&e, dst),
            &rust(eye, target, up),
            &format!("lookat[{i}] eye={eye:?} target={target:?} up={up:?}"),
        );
    }
}

#[test]
fn lookat_v1_difftest() {
    lookat_check(0x5c3e_0001, LOOKAT_V1, lookat_v1);
}
