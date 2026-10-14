//! The 3×3 determinants, adjugate and inverse (`systems/collision/airborne.md#crease-helpers`) and
//! the 3×3 row scales. A matrix is nine `f32`, row-major.

/// `0x7bc040` (the nine entries by value on the stack, `ret 0x24`; the result in `st0`): the
/// determinant as six products of three, `m1·m5·m6 + m2·m3·m7 + m0·m4·m8 − m2·m4·m6 − m1·m3·m8 −
/// m0·m5·m7` in that order. This is the register at 53 bits; the caller's store rounds it to `f32`,
/// and the 4×4 determinant `0x7bcf90` and adjugate `0x7bd390` read it before that store.
#[must_use]
pub fn det3_f64(m: &[f32; 9]) -> f64 {
    let f = f64::from;
    f(m[1]) * f(m[5]) * f(m[6]) + f(m[2]) * f(m[3]) * f(m[7]) + f(m[0]) * f(m[4]) * f(m[8])
        - f(m[2]) * f(m[4]) * f(m[6])
        - f(m[1]) * f(m[3]) * f(m[8])
        - f(m[0]) * f(m[5]) * f(m[7])
}

/// `0x7bc040` as the `f32` a caller stores from `st0`.
#[must_use]
pub fn det3(m: &[f32; 9]) -> f32 {
    det3_f64(m) as f32
}

/// `0x7be6d0` (`ecx` = the matrix, the stack argument a vector of three; `ret 4`): every entry of
/// row `r` times `v[r]`, in place, each product stored as `f32`.
#[must_use]
pub fn mat3_scale_rows(m: &[f32; 9], v: &[f32; 3]) -> [f32; 9] {
    core::array::from_fn(|i| (f64::from(v[i / 3]) * f64::from(m[i])) as f32)
}

/// `0x7be670` (`ecx` = the matrix, three scalars on the stack; `ret 0xc`): `mat3_scale_rows` with
/// the three row factors passed by value.
#[must_use]
pub fn mat3_scale_rows3(m: &[f32; 9], s: &[f32; 3]) -> [f32; 9] {
    mat3_scale_rows(m, s)
}

/// `Matrix3Determinant` (`0x7be0e0`, `ecx` = the matrix, the result in `st0`): the rule of
/// Sarrus, three positive products summed and three negative ones subtracted, in the order below.
#[must_use]
pub fn det3b(m: &[f32; 9]) -> f32 {
    let f = f64::from;
    (f(m[6]) * f(m[1]) * f(m[5]) + f(m[2]) * f(m[3]) * f(m[7]) + f(m[8]) * f(m[0]) * f(m[4])
        - f(m[6]) * f(m[2]) * f(m[4])
        - f(m[8]) * f(m[1]) * f(m[3])
        - f(m[0]) * f(m[7]) * f(m[5])) as f32
}

/// `Matrix3Adjugate` (`0x7be1e0`, `ecx` = the matrix, `[ebp+8]` = the output): the transposed
/// cofactor matrix, each element one difference of two products, rounded to `f32`.
#[must_use]
pub fn adj3t(m: &[f32; 9]) -> [f32; 9] {
    let f = f64::from;
    [
        (f(m[8]) * f(m[4]) - f(m[5]) * f(m[7])) as f32,
        (-(f(m[1]) * f(m[8]) - f(m[2]) * f(m[7]))) as f32,
        (f(m[5]) * f(m[1]) - f(m[2]) * f(m[4])) as f32,
        (-(f(m[3]) * f(m[8]) - f(m[5]) * f(m[6]))) as f32,
        (f(m[8]) * f(m[0]) - f(m[2]) * f(m[6])) as f32,
        (-(f(m[5]) * f(m[0]) - f(m[2]) * f(m[3]))) as f32,
        (f(m[7]) * f(m[3]) - f(m[6]) * f(m[4])) as f32,
        (-(f(m[7]) * f(m[0]) - f(m[1]) * f(m[6]))) as f32,
        (f(m[0]) * f(m[4]) - f(m[1]) * f(m[3])) as f32,
    ]
}

/// `InvertMatrix3` (`0x7be2a0`, `ecx` = the matrix, `[ebp+8]` = the output, `[ebp+0xc]` = the
/// determinant `s` by value): `1/s` rounded to `f32`, times each element of the adjugate
/// `0x7be1e0` writes, each product rounded to `f32`. There is no zero test.
#[must_use]
pub fn inv3(m: &[f32; 9], s: f32) -> [f32; 9] {
    let f = f64::from;
    let adj = adj3t(m);
    let inv_s = (1.0_f64 / f(s)) as f32;
    let mut out = [0f32; 9];
    for i in 0..9 {
        out[i] = (f(inv_s) * f(adj[i])) as f32;
    }
    out
}
