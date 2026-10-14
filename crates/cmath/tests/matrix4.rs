//! The 4×4 routines against the exe, each run whole: the adjugate `0x7bd390`, the determinant
//! `0x7bcf90`, the sum `0x7bc290`, the scalar product `0x7bc8e0`, the inequality test `0x7bc1a0`,
//! the inverse from a determinant `0x7bd6c0` and the affine inverses `0x7bd700` and `0x7bd820`.
//! The inputs are the edge fills, a mix, and random non-NaN bit patterns; the scalars of the
//! affine inverse add the two sides of the `2^-20` guard. A NaN result matches any NaN.

mod common;

use benilla_re_cmath::matrix4 as m4;
use common::*;

#[test]
fn adj4t() {
    check_unary("adj4t", &[0x7bd390], 16, 16, |m| m4::adj4t(arr(m)).to_vec());
}

#[test]
fn det4() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(16);
    for m in &mats {
        d.put(d.a, m);
        let got = d.emu.call_f32(0x7bcf90, d.a as u32, 0, &[]); // 4×4 matrix by pointer in ecx
        assert_f32(got, m4::det4(arr(m.as_slice())), "det4");
    }
    eprintln!("det4: {} cases bit-exact vs WoW.exe @ 0x7bcf90", mats.len());
}

#[test]
fn mat4_add() {
    check_binop("mat4_add", &[0x7bc290], 16, |a, b| {
        m4::mat4_add(arr(a), arr(b)).to_vec()
    });
}

#[test]
fn mat4_mul_scalar() {
    check_scalarop("mat4_mul_scalar", &[0x7bc8e0], 16, |a, s| {
        m4::mat4_mul_scalar(arr(a), s).to_vec()
    });
}

#[test]
fn mat4_ne() {
    check_cmp("mat4_ne", &[0x7bc1a0], 16, |a, b| {
        m4::mat4_ne(arr(a), arr(b))
    });
}

#[test]
fn mat4_inverse_from_det() {
    check_mat_scalar_build(
        "mat4_inverse_from_det",
        &[0x7bd6c0],
        16,
        16,
        &synth_scalars(),
        |m, s| m4::mat4_inverse_from_det(arr(m), s).to_vec(),
    );
}

#[test]
fn mat4_affine_invert() {
    check_mat_inout("mat4_affine_invert", &[0x7bd700], 16, 16, |m| {
        m4::mat4_affine_invert(arr(m)).to_vec()
    });
}

#[test]
fn mat4_affine_invert_uscale() {
    let mut scalars = synth_scalars();
    scalars.extend_from_slice(&[
        1.0,
        1.0 + 2f32.powi(-21),
        1.0 - 2f32.powi(-21),
        1.0 + 2f32.powi(-19),
        1.0 - 2f32.powi(-19),
    ]);
    check_mat_scalar_build(
        "mat4_affine_invert_uscale",
        &[0x7bd820],
        16,
        16,
        &scalars,
        |m, s| m4::mat4_affine_invert_uscale(arr(m), s).to_vec(),
    );
}
