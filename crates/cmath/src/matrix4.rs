//! The 4×4 routines: the determinant, the adjugate and the inverse built from them, the element
//! sum and scalar product, the inequality test, and the inverses of a rotation with a translation.
//! A matrix is sixteen `f32`, row-major (`m[4 * row + column]`); the translation is `m[12..15]`.

use crate::matrix3::det3_f64;

/// The 3×3 minor that deletes the rows and columns not listed, as the nine entries
/// `0x7bc040` reads.
fn minor(m: &[f32; 16], rows: [usize; 3], cols: [usize; 3]) -> f64 {
    let at = |r: usize, c: usize| m[4 * rows[r] + cols[c]];
    det3_f64(&[
        at(0, 0),
        at(0, 1),
        at(0, 2),
        at(1, 0),
        at(1, 1),
        at(1, 2),
        at(2, 0),
        at(2, 1),
        at(2, 2),
    ])
}

/// `Determinant4` (`0x7bcf90`, `ecx` = the matrix, the result in `st0`): the expansion along row 0,
/// `m0·D0 − m1·D1 + m2·D2 − m3·D3`, where `Dj` is the 3×3 determinant `0x7bc040` leaves in `st0`
/// at 53 bits for the minor without row 0 and column `j`. The running sum is stored to an `f32`
/// slot after every term, so each partial sum rounds to `f32`.
#[must_use]
pub fn det4(m: &[f32; 16]) -> f32 {
    let f = f64::from;
    let d = |col: [usize; 3]| minor(m, [1, 2, 3], col);
    let acc = (d([1, 2, 3]) * f(m[0])) as f32;
    let acc = (f(acc) - d([0, 2, 3]) * f(m[1])) as f32;
    let acc = (f(acc) + d([0, 1, 3]) * f(m[2])) as f32;
    (f(acc) - d([0, 1, 2]) * f(m[3])) as f32
}

/// `0x7bd390` (`ecx` = the matrix, `[ebp+8]` = the output): the adjugate. Output `[4i + j]` is
/// `(−1)^(i+j)` times the 3×3 determinant `0x7bc040` leaves in `st0` for the minor without row `j`
/// and column `i`; an odd position negates the register, and the store rounds to `f32`.
#[must_use]
pub fn adj4t(m: &[f32; 16]) -> [f32; 16] {
    const KEEP: [[usize; 3]; 4] = [[1, 2, 3], [0, 2, 3], [0, 1, 3], [0, 1, 2]];
    core::array::from_fn(|k| {
        let (i, j) = (k / 4, k % 4);
        let d = minor(m, KEEP[j], KEEP[i]);
        (if (i + j) % 2 == 1 { -d } else { d }) as f32
    })
}

/// `InvertMatrix4WithDeterminant` (`0x7bd6c0`, `ecx` = the matrix, `[ebp+8]` = the output,
/// `[ebp+0xc]` = the determinant `s` by value): `1/s` rounded to `f32`, times every entry of the
/// adjugate `0x7bd390`, through `mat4_mul_scalar`. There is no zero test.
#[must_use]
pub fn mat4_inverse_from_det(m: &[f32; 16], s: f32) -> [f32; 16] {
    let inv = (1.0_f64 / f64::from(s)) as f32;
    mat4_mul_scalar(&adj4t(m), inv)
}

/// `0x7bc290` (`ecx` = the output, `edx` = a, the stack argument b): the entries summed in pairs,
/// each sum stored as `f32`.
#[must_use]
pub fn mat4_add(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    core::array::from_fn(|i| (f64::from(a[i]) + f64::from(b[i])) as f32)
}

/// `0x7bc8e0` (`ecx` = the output, `edx` = the matrix, the scalar `s` on the stack): `s` times
/// every entry, each product stored as `f32`.
#[must_use]
pub fn mat4_mul_scalar(a: &[f32; 16], s: f32) -> [f32; 16] {
    core::array::from_fn(|i| (f64::from(s) * f64::from(a[i])) as f32)
}

/// `0x7bc1a0` (`ecx` = a, `edx` = b; the answer in `al`): whether any pair of entries differs
/// under the float comparison, so `−0` equals `0` and a NaN differs from everything.
#[must_use]
pub fn mat4_ne(a: &[f32; 16], b: &[f32; 16]) -> bool {
    a.iter().zip(b).any(|(x, y)| x != y)
}

/// The upper 3×3 of a 4×4, row by row.
fn rotation_of(m: &[f32; 16]) -> [f32; 9] {
    [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]]
}

fn transpose3(p: &[f32; 9]) -> [f32; 9] {
    [p[0], p[3], p[6], p[1], p[4], p[7], p[2], p[5], p[8]]
}

/// A 4×4 with `rot` in its upper 3×3 and `(0, 0, 0, 1)` as the last row and column, then the
/// negated translation `t` taken through that 3×3 into row 3 with `0x7bdc40`.
fn inverse_with_translation(rot: &[f32; 9], t: &[f32; 3]) -> [f32; 16] {
    let mut out = [
        rot[0], rot[1], rot[2], 0.0, rot[3], rot[4], rot[5], 0.0, rot[6], rot[7], rot[8], 0.0, 0.0,
        0.0, 0.0, 1.0,
    ];
    crate::transform::translate_matrix(&mut out, &[-t[0], -t[1], -t[2]]);
    out
}

/// `InvertRigidMatrix` (`0x7bd700`, `ecx` = the matrix, `[ebp+8]` = the output): the inverse of a
/// rotation with a translation, the transposed 3×3 with the negated translation carried through it.
#[must_use]
pub fn mat4_affine_invert(m: &[f32; 16]) -> [f32; 16] {
    inverse_with_translation(&transpose3(&rotation_of(m)), &[m[12], m[13], m[14]])
}

/// `InvertAffineMatrix` (`0x7bd820`, `ecx` = the matrix, `[ebp+8]` = the output, `[ebp+0xc]` = the
/// uniform scale `s` by value): `0x7bd700` when `|s − 1| < 2^-20` (the constant at `0x8026bc`);
/// otherwise the transposed 3×3 with every entry times `1/(s·s)` (the product and the quotient at 53
/// bits, the quotient stored as `f32`), by `0x7bdd00`, then the negated translation carried through
/// the scaled 3×3.
#[must_use]
pub fn mat4_affine_invert_uscale(m: &[f32; 16], s: f32) -> [f32; 16] {
    const EPSILON: f64 = 1.0 / 1_048_576.0;
    let mut rot = transpose3(&rotation_of(m));
    if (f64::from(s) - 1.0).abs() >= EPSILON {
        let recip = (1.0_f64 / (f64::from(s) * f64::from(s))) as f32;
        for v in &mut rot {
            *v = (f64::from(recip) * f64::from(*v)) as f32;
        }
    }
    inverse_with_translation(&rot, &[m[12], m[13], m[14]])
}
