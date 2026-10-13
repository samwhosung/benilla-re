//! A rotation matrix to a quaternion (`0x7c0190`), by the trace and the largest diagonal.
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
