//! The rotation-and-translation matrices of twelve `f32`: the 3×3 rotation row by row (`m[0..9]`)
//! followed by the translation (`m[9..12]`).

/// `0x7bae60` (`ecx` = the output, `edx` = a, the stack argument b): the product of two
/// rotation-and-translation matrices stored as nine entries then the translation. The 3×3 part is
/// the 3×3 product, each output summed in its own order; the translation is `t_a · B + t_b` with
/// the last term added last. Each result is stored as `f32`.
#[must_use]
pub fn affine_mul34(a: &[f32; 12], b: &[f32; 12]) -> [f32; 12] {
    let f = f64::from;
    [
        (f(a[0]) * f(b[0]) + f(a[2]) * f(b[6]) + f(a[1]) * f(b[3])) as f32,
        (f(a[2]) * f(b[7]) + f(a[1]) * f(b[4]) + f(b[1]) * f(a[0])) as f32,
        (f(a[1]) * f(b[5]) + f(a[2]) * f(b[8]) + f(a[0]) * f(b[2])) as f32,
        (f(b[6]) * f(a[5]) + f(b[3]) * f(a[4]) + f(a[3]) * f(b[0])) as f32,
        (f(a[3]) * f(b[1]) + f(a[5]) * f(b[7]) + f(a[4]) * f(b[4])) as f32,
        (f(b[8]) * f(a[5]) + f(a[3]) * f(b[2]) + f(a[4]) * f(b[5])) as f32,
        (f(a[6]) * f(b[0]) + f(b[6]) * f(a[8]) + f(b[3]) * f(a[7])) as f32,
        (f(a[8]) * f(b[7]) + f(a[7]) * f(b[4]) + f(a[6]) * f(b[1])) as f32,
        (f(a[8]) * f(b[8]) + f(a[6]) * f(b[2]) + f(a[7]) * f(b[5])) as f32,
        (f(b[6]) * f(a[11]) + f(a[10]) * f(b[3]) + f(a[9]) * f(b[0]) + f(b[9])) as f32,
        (f(a[10]) * f(b[4]) + f(a[9]) * f(b[1]) + f(a[11]) * f(b[7]) + f(b[10])) as f32,
        (f(a[10]) * f(b[5]) + f(a[9]) * f(b[2]) + f(b[8]) * f(a[11]) + f(b[11])) as f32,
    ]
}

/// `0x7bb0e0` (`ecx` = the output, `edx` = the point, the stack argument the matrix): the point
/// times the 3×3 plus the translation `b[9..12]`, each output `((b6·z + b3·y) + x·b0) + b9` for x
/// and likewise for y and z, stored as `f32`.
#[must_use]
pub fn transform_point34(a: &[f32; 3], b: &[f32; 12]) -> [f32; 3] {
    let f = f64::from;
    [
        (f(b[6]) * f(a[2]) + f(b[3]) * f(a[1]) + f(a[0]) * f(b[0]) + f(b[9])) as f32,
        (f(b[7]) * f(a[2]) + f(b[4]) * f(a[1]) + f(b[1]) * f(a[0]) + f(b[10])) as f32,
        (f(b[8]) * f(a[2]) + f(b[5]) * f(a[1]) + f(b[2]) * f(a[0]) + f(b[11])) as f32,
    ]
}

/// `0x7bb990` (`ecx` = the matrix, changed in place, the stack argument the vector): the 3×3 times
/// the vector added to the translation `m[9..12]`, `((m3·y + m6·z) + x·m0) + m9` for x and likewise
/// for y and z, stored as `f32`; the other nine entries are unchanged.
#[must_use]
pub fn apply_translation3(a: &[f32; 12], b: &[f32; 3]) -> [f32; 12] {
    let f = f64::from;
    let mut d = *a;
    d[9] = (f(a[3]) * f(b[1]) + f(a[6]) * f(b[2]) + f(b[0]) * f(a[0]) + f(a[9])) as f32;
    d[10] = (f(a[4]) * f(b[1]) + f(a[1]) * f(b[0]) + f(a[7]) * f(b[2]) + f(a[10])) as f32;
    d[11] = (f(a[5]) * f(b[1]) + f(a[2]) * f(b[0]) + f(a[8]) * f(b[2]) + f(a[11])) as f32;
    d
}
