//! The 3×3 determinant, adjugate and inverse (`systems/collision/airborne.md#crease-helpers`). A
//! matrix is nine `f32`, row-major.

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
