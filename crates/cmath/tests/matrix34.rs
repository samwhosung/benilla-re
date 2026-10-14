//! The rotation-and-translation matrices of twelve entries against the exe, each run whole: the
//! product `0x7bae60`, a point through one `0x7bb0e0` and the translation update `0x7bb990`. The
//! inputs are the edge fills, a mix, and random non-NaN bit patterns. A NaN result matches any
//! NaN.

mod common;

use benilla_re_cmath::matrix34 as m34;
use common::*;

#[test]
fn affine_mul34() {
    check_binop("affine_mul34", &[0x7bae60], 12, |a, b| {
        m34::affine_mul34(arr(a), arr(b)).to_vec()
    });
}

#[test]
fn transform_point34() {
    check_vecmat("transform_point34", &[0x7bb0e0], 3, 12, 3, |a, b| {
        m34::transform_point34(arr(a), arr(b)).to_vec()
    });
}

#[test]
fn apply_translation3() {
    check_vecscale_inplace("apply_translation3", &[0x7bb990], 12, 3, |a, b| {
        m34::apply_translation3(arr(a), arr(b)).to_vec()
    });
}
