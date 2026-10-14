//! A rotation matrix to a quaternion (`0x7c0190`), by the trace and the largest diagonal, and a
//! quaternion's rotation applied on the left of a 4×4 (`0x7bddb0`).
//!
//! `__thiscall`: `ecx` = the output quaternion `[x, y, z, w]`, `[ebp+8]` = a row-major 3×3
//! matrix, element `m[r][c]` at index `r*3 + c`. The constants are 0.5 (`0x3f000000`), 0.0 and
//! 1.0, and the index cycle `{1, 2, 0}` at `0x81d994`. Each component is rounded to `f32` at its
//! store; the sum, the square root and `0.5/s` stay in the register.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// A quaternion `[x, y, z, w]`, at `+0`, `+4`, `+8` and `+0xc`.
pub type Quat = [f32; 4];

/// `[0x81d994]`: the next index, `(i + 1) mod 3`.
const NEXT: [usize; 3] = [1, 2, 0];

/// `0x7c0190`: the trace `S = (m11 + m00) + m22`. For `S > 0`, `s = √(S + 1)`, `w = 0.5·s` and
/// the other three are differences of opposite elements times `0.5/s`. Otherwise (`S ≤ 0` or
/// NaN) `i` is the largest diagonal, the earlier index on a tie or NaN, `j` and `k` the next two,
/// `s = √(((m_ii − m_jj) − m_kk) + 1)`, `q_i = 0.5·s`, and the rest use `0.5/s`.
pub fn matrix_to_quat(m: &[f32; 9]) -> Quat {
    let diag = [f(m[0]), f(m[4]), f(m[8])];
    let trace = (diag[1] + diag[0]) + diag[2];

    let mut out: Quat = [0.0; 4];

    if trace > 0.0 {
        let s = (trace + 1.0).sqrt();
        out[3] = (0.5 * s) as f32;
        let scale = 0.5 / s;
        out[0] = ((f(m[7]) - f(m[5])) * scale) as f32;
        out[1] = ((f(m[2]) - f(m[6])) * scale) as f32;
        out[2] = ((f(m[3]) - f(m[1])) * scale) as f32;
    } else {
        let mut i = 0usize;
        if diag[1] > diag[0] {
            i = 1;
        }
        if diag[2] > diag[i] {
            i = 2;
        }
        let j = NEXT[i];
        let k = NEXT[j];

        let s = (((diag[i] - diag[j]) - diag[k]) + 1.0).sqrt();
        out[i] = (0.5 * s) as f32;
        let scale = 0.5 / s;

        out[3] = ((f(m[k * 3 + j]) - f(m[j * 3 + k])) * scale) as f32;
        out[j] = ((f(m[i * 3 + j]) + f(m[j * 3 + i])) * scale) as f32;
        out[k] = ((f(m[i * 3 + k]) + f(m[k * 3 + i])) * scale) as f32;
    }

    out
}

/// The rotation matrix `0x7bddb0` builds from `q`, row by row: `2z` is stored to an `f32` slot
/// first, then `2xw`, `2yw`, `2zw`, `2x²`, `2xy`, `2xz` and `2yz` are each stored as `f32`, while
/// `2y²` stays in the register and `2z²` is used both as the register (`m0`) and as the `f32` store
/// (`m4`). The entries are `1 − (2z² + 2y²)`, `2xy + 2zw`, `2xz − 2yw`, `2xy − 2zw`, `1 − (2z² +
/// 2x²)`, `2yz + 2xw`, `2xz + 2yw`, `2yz − 2xw` and `1 − (2y² + 2x²)`, each stored as `f32`.
fn quaternion_rotation(q: &Quat) -> [f32; 9] {
    let (x, y, z, w) = (f(q[0]), f(q[1]), f(q[2]), f(q[3]));
    let tx = 2.0 * x;
    let ty = 2.0 * y;
    let tz = f((2.0 * z) as f32);
    let l2xw = f((tx * w) as f32);
    let l2yw = f((ty * w) as f32);
    let l2zw = f((tz * w) as f32);
    let l2x2 = f((tx * x) as f32);
    let l2xy = f((ty * x) as f32);
    let l2xz = f((tz * x) as f32);
    let l2yz = f((tz * y) as f32);
    let r2y2 = ty * y;
    let r2z2 = tz * z;
    let f2z2 = f((tz * z) as f32);
    [
        (1.0 - (r2z2 + r2y2)) as f32,
        (l2xy + l2zw) as f32,
        (l2xz - l2yw) as f32,
        (l2xy - l2zw) as f32,
        (1.0 - (f2z2 + l2x2)) as f32,
        (l2yz + l2xw) as f32,
        (l2xz + l2yw) as f32,
        (l2yz - l2xw) as f32,
        (1.0 - (r2y2 + l2x2)) as f32,
    ]
}

/// `RotateMatrixByQuaternion` (`0x7bddb0`, `ecx` = the matrix, changed in place, `[ebp+8]` = the
/// quaternion `[x, y, z, w]`): builds the rotation above in a 4×4 whose fourth row and column are
/// `(0, 0, 0, 1)`, then replaces the matrix with `rotation · matrix` by `MultiplyMatrix4`.
#[must_use]
pub fn quat_mul4(m: &[f32; 16], q: &Quat) -> [f32; 16] {
    let r = quaternion_rotation(q);
    let rotation = [
        r[0], r[1], r[2], 0.0, r[3], r[4], r[5], 0.0, r[6], r[7], r[8], 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    crate::multiply::multiply_matrix4(&rotation, m)
}
