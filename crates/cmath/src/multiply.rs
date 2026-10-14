//! The 3×3 and 4×4 matrix products and the dot leaf the 4×4 product opens with. A matrix is `f32`,
//! row-major; each output is a sum of products taken at 53 bits and stored as `f32`, in the order
//! the exe's x87 code takes them.

/// `0x7bc8c0` (`ecx` = a vector of four, `edx` = a 4×4; returns in `st0`): `a·column 0` of `b` as
/// `((b12·a3 + b8·a2) + a1·b4) + a0·b0`, rounded to `f32` by the caller's store.
pub fn dot_leaf(a: &[f32; 4], b: &[f32; 16]) -> f32 {
    let f = f64::from;
    (f(b[12]) * f(a[3]) + f(b[8]) * f(a[2]) + f(a[1]) * f(b[4]) + f(a[0]) * f(b[0])) as f32
}

/// `0x7bdfc0` (`ecx` = out, `edx` = a, the stack argument b): the product of two 3×3 matrices,
/// `out[3i + j]` the row `i` of `a` against the column `j` of `b`. The three products of each
/// output are summed in the order the x87 code interleaves them, which differs per output.
pub fn multiply_matrix3(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    let f = f64::from;
    [
        (f(b[6]) * f(a[2]) + f(b[3]) * f(a[1]) + f(a[0]) * f(b[0])) as f32,
        (f(b[1]) * f(a[0]) + f(b[7]) * f(a[2]) + f(a[1]) * f(b[4])) as f32,
        (f(b[8]) * f(a[2]) + f(a[0]) * f(b[2]) + f(a[1]) * f(b[5])) as f32,
        (f(a[3]) * f(b[0]) + f(b[6]) * f(a[5]) + f(b[3]) * f(a[4])) as f32,
        (f(a[5]) * f(b[7]) + f(a[4]) * f(b[4]) + f(a[3]) * f(b[1])) as f32,
        (f(a[5]) * f(b[8]) + f(a[3]) * f(b[2]) + f(a[4]) * f(b[5])) as f32,
        (f(b[6]) * f(a[8]) + f(a[7]) * f(b[3]) + f(a[6]) * f(b[0])) as f32,
        (f(a[7]) * f(b[4]) + f(a[6]) * f(b[1]) + f(a[8]) * f(b[7])) as f32,
        (f(a[7]) * f(b[5]) + f(a[6]) * f(b[2]) + f(b[8]) * f(a[8])) as f32,
    ]
}

/// `0x7bc6a0` (`ecx` = out, `edx` = a, the stack argument b): the product of two 4×4 matrices,
/// `out[4i + j]` the row `i` of `a` against the column `j` of `b`. Element 0 is `dot_leaf`
/// (`0x7bc8c0`) of row 0; the other fifteen are summed inline, each in its own order.
pub fn multiply_matrix4(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let f = f64::from;
    [
        dot_leaf(&[a[0], a[1], a[2], a[3]], b),
        (f(a[3]) * f(b[13]) + f(a[2]) * f(b[9]) + f(b[5]) * f(a[1]) + f(b[1]) * f(a[0])) as f32,
        (f(a[3]) * f(b[14]) + f(a[2]) * f(b[10]) + f(b[6]) * f(a[1]) + f(b[2]) * f(a[0])) as f32,
        (f(a[2]) * f(b[11]) + f(a[3]) * f(b[15]) + f(a[0]) * f(b[3]) + f(b[7]) * f(a[1])) as f32,
        (f(a[7]) * f(b[12]) + f(a[6]) * f(b[8]) + f(a[4]) * f(b[0]) + f(b[4]) * f(a[5])) as f32,
        (f(a[7]) * f(b[13]) + f(a[6]) * f(b[9]) + f(b[5]) * f(a[5]) + f(b[1]) * f(a[4])) as f32,
        (f(a[7]) * f(b[14]) + f(a[6]) * f(b[10]) + f(b[6]) * f(a[5]) + f(b[2]) * f(a[4])) as f32,
        (f(a[6]) * f(b[11]) + f(a[7]) * f(b[15]) + f(a[4]) * f(b[3]) + f(a[5]) * f(b[7])) as f32,
        (f(a[11]) * f(b[12]) + f(a[10]) * f(b[8]) + f(a[8]) * f(b[0]) + f(b[4]) * f(a[9])) as f32,
        (f(a[11]) * f(b[13]) + f(a[10]) * f(b[9]) + f(b[5]) * f(a[9]) + f(b[1]) * f(a[8])) as f32,
        (f(a[11]) * f(b[14]) + f(a[10]) * f(b[10]) + f(b[6]) * f(a[9]) + f(b[2]) * f(a[8])) as f32,
        (f(a[10]) * f(b[11]) + f(a[11]) * f(b[15]) + f(a[8]) * f(b[3]) + f(a[9]) * f(b[7])) as f32,
        (f(a[14]) * f(b[8]) + f(a[13]) * f(b[4]) + f(a[15]) * f(b[12]) + f(a[12]) * f(b[0])) as f32,
        (f(a[14]) * f(b[9]) + f(a[12]) * f(b[1]) + f(a[13]) * f(b[5]) + f(a[15]) * f(b[13])) as f32,
        (f(a[14]) * f(b[10]) + f(a[12]) * f(b[2]) + f(a[13]) * f(b[6]) + f(a[15]) * f(b[14]))
            as f32,
        (f(a[14]) * f(b[11]) + f(a[13]) * f(b[7]) + f(a[12]) * f(b[3]) + f(b[15]) * f(a[15]))
            as f32,
    ]
}
