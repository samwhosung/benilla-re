//! `TransformPoint` (`0x7bca80`, `systems/collision/falling.md#frame-transform`),
//! `TransformVector4` (`0x7bcb40`) and `ScaleMatrixRows` (`0x7bdca0`,
//! `systems/camera.md#orthographic-setup`).

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
