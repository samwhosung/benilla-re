//! A point and a 4-vector through a 4×4 (`0x7bca80`, `0x7bcc60` and `0x7bcb40`) and the row and
//! uniform scales of a 4×4 (`0x7bdca0` and `0x7bdd00`), each run whole. The inputs are the edge
//! fills, a mix, and random non-NaN bit patterns. A NaN result matches any NaN.

mod common;

use benilla_re_cmath::transform as tf;
use common::*;

/// `0x7bca80` and its in-place twin `0x7bcc60` take the output, the point and the matrix in the
/// same registers, and give the same result.
#[test]
fn transform_point4() {
    check_vecmat(
        "transform_point4",
        &[0x7bca80, 0x7bcc60],
        3,
        16,
        3,
        |a, b| tf::transform_point(arr(a), arr(b)).to_vec(),
    );
}

#[test]
fn transform_vec4() {
    check_vecmat("transform_vec4", &[0x7bcb40], 4, 16, 4, |a, b| {
        tf::transform_vec4(arr(a), arr(b)).to_vec()
    });
}

/// `0x7bdca0` on a matrix of sixteen: only the first three entries of rows 0 to 2 change.
#[test]
fn mat34_scale3_rows() {
    check_vecscale_inplace("mat34_scale3_rows", &[0x7bdca0], 16, 3, |m, v| {
        let mut out = *arr::<16>(m);
        tf::scale_matrix_rows(&mut out, *arr::<3>(v));
        out.to_vec()
    });
}

/// `0x7bdd00` on a matrix of sixteen: only the first three entries of rows 0 to 2 change.
#[test]
fn mat34_scale3() {
    check_scale_inplace("mat34_scale3", &[0x7bdd00], 16, |m, s| {
        let mut out = *arr::<16>(m);
        tf::scale_matrix(&mut out, s);
        out.to_vec()
    });
}
