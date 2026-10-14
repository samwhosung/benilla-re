//! `TransformPoint` (`0x7bca80`, `systems/collision/falling.md#frame-transform`),
//! `TransformVector4` (`0x7bcb40`), `ScaleMatrixRows` (`0x7bdca0`,
//! `systems/camera.md#orthographic-setup`) and `ScaleMatrix` (`0x7bdd00`).

/// `0x7bca80`: `v` times the upper 3×3 of the 4×4 `m` plus its fourth row: `x = ((m8·z + m4·y) +
/// x·m0) + m12`, `y = ((m9·z + m1·x) + m5·y) + m13`, `z = ((m10·z + m2·x) + m6·y) + m14`, each
/// stored as `f32`; z is computed first.
pub fn transform_point(v: &[f32; 3], m: &[f32; 16]) -> [f32; 3] {
    let f = f64::from;
    [
        (f(m[8]) * f(v[2]) + f(m[4]) * f(v[1]) + f(v[0]) * f(m[0]) + f(m[12])) as f32,
        (f(m[9]) * f(v[2]) + f(m[1]) * f(v[0]) + f(m[5]) * f(v[1]) + f(m[13])) as f32,
        (f(m[10]) * f(v[2]) + f(m[2]) * f(v[0]) + f(m[6]) * f(v[1]) + f(m[14])) as f32,
    ]
}

/// `0x7bcb40`: the 4-vector `v` times the 4×4 `m`: component `i` is `((m[12+i]·w + m[i]·x) +
/// m[8+i]·z) + m[4+i]·y` for y, z and w, and `((m12·w + m8·z) + m4·y) + x·m0` for x, each stored as
/// `f32`; w is computed first.
pub fn transform_vec4(v: &[f32; 4], m: &[f32; 16]) -> [f32; 4] {
    let f = f64::from;
    [
        (f(m[12]) * f(v[3]) + f(m[8]) * f(v[2]) + f(m[4]) * f(v[1]) + f(v[0]) * f(m[0])) as f32,
        (f(m[13]) * f(v[3]) + f(m[1]) * f(v[0]) + f(m[9]) * f(v[2]) + f(m[5]) * f(v[1])) as f32,
        (f(m[14]) * f(v[3]) + f(m[2]) * f(v[0]) + f(m[10]) * f(v[2]) + f(m[6]) * f(v[1])) as f32,
        (f(m[15]) * f(v[3]) + f(m[3]) * f(v[0]) + f(m[11]) * f(v[2]) + f(m[7]) * f(v[1])) as f32,
    ]
}

/// `0x7bdca0`: the first three entries of rows 0 to 2 of the 4×4 `m` times `s[row]`, each product
/// stored as `f32`.
pub fn scale_matrix_rows(m: &mut [f32; 16], s: [f32; 3]) {
    for (row, &scalar) in s.iter().enumerate() {
        for v in &mut m[row * 4..row * 4 + 3] {
            *v = (f64::from(scalar) * f64::from(*v)) as f32;
        }
    }
}

/// `0x7bcae0` (`ecx` = out, `edx` = the matrix of three rows of four, the stack argument the
/// vector; `ret 4`): row `i` dotted with `v`, plus the row's fourth entry: `((m2·z + m1·y) + m0·x) + m3`
/// for x, `((m6·z + m4·x) + m5·y) + m7` for y and `((m10·z + m8·x) + m9·y) + m11` for z, each
/// stored as `f32`.
pub fn transform_mat43_vec3(m: &[f32; 12], v: &[f32; 3]) -> [f32; 3] {
    let f = f64::from;
    [
        (f(m[2]) * f(v[2]) + f(m[1]) * f(v[1]) + f(m[0]) * f(v[0]) + f(m[3])) as f32,
        (f(m[6]) * f(v[2]) + f(m[4]) * f(v[0]) + f(m[5]) * f(v[1]) + f(m[7])) as f32,
        (f(m[10]) * f(v[2]) + f(m[8]) * f(v[0]) + f(m[9]) * f(v[1]) + f(m[11])) as f32,
    ]
}

/// `0x7bdc40` (`ecx` = the matrix, the stack argument the translation `t`; `ret 4`): row 3 plus
/// `t[0]·row 0 + t[1]·row 1 + t[2]·row 2`, per component `((m8·t2 + m4·t1) + t0·m0) + m12` for x,
/// `((m9·t2 + m5·t1) + m1·t0) + m13` for y and `((m10·t2 + m6·t1) + m2·t0) + m14` for z, each stored
/// as `f32`; the other entries are unchanged.
pub fn translate_matrix(m: &mut [f32; 16], t: &[f32; 3]) {
    let f = f64::from;
    m[12] = (f(m[8]) * f(t[2]) + f(m[4]) * f(t[1]) + f(t[0]) * f(m[0]) + f(m[12])) as f32;
    m[13] = (f(m[9]) * f(t[2]) + f(m[5]) * f(t[1]) + f(m[1]) * f(t[0]) + f(m[13])) as f32;
    m[14] = (f(m[10]) * f(t[2]) + f(m[6]) * f(t[1]) + f(m[2]) * f(t[0]) + f(m[14])) as f32;
}

/// `ScaleMatrix` (`0x7bdd00`, `ecx` = the matrix, the scalar on the stack): the first three
/// entries of rows 0 to 2 of the 4×4 `m` times `s`, in place, each product stored as `f32`. Entry 3
/// of each row, and row 3, are unchanged.
pub fn scale_matrix(m: &mut [f32; 16], s: f32) {
    for row in 0..3 {
        for v in &mut m[row * 4..row * 4 + 3] {
            *v = (f64::from(s) * f64::from(*v)) as f32;
        }
    }
}
