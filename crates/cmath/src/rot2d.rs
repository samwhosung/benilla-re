//! A 2×2 rotation from an angle (`0x7c42f0`).
//!
//! `__thiscall`: `ecx` = the output, four `f32`; `[ebp+8]` = the angle. `fsincos` gives the
//! cosine and the sine, each rounded to `f32` at its store; the result is
//! `[cos, sin, −sin, cos]`, the negation exact. The emulator evaluates `fsincos` with the host's
//! maths library, as `f64::cos` and `f64::sin` do here (`METHOD.md`, The binary decides).

/// `0x7c42f0`: `[[cos θ, sin θ], [−sin θ, cos θ]]`, row-major.
pub fn rot2d(angle: f32) -> [f32; 4] {
    let a = f64::from(angle);
    let cos = a.cos() as f32;
    let sin = a.sin() as f32;
    [cos, sin, -sin, cos]
}
