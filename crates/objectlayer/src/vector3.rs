//! The two `C3Vector` leaves that sit in the object layer's code: `ScaleVector3` (`0x5f8cf0`) and
//! `AddVector3` (`0x621160`), called throughout the client (`systems/camera.md#unprojection`).
//!
//! Each loads `f32` operands, computes at 53 bits and stores a result with `fstp dword`, so it is
//! an `f64` operation with a final `as f32`. Both take `ecx` as the output vector, `edx` as the
//! first operand and one stack argument, and return with `ret 4`.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `0x5f8cf0`: `out[i] = s * v[i]`, with `v` in `edx` and `s` the stack argument.
pub fn vec_scale(v: &[f32; 3], s: f32) -> [f32; 3] {
    let s = f(s);
    [
        (s * f(v[0])) as f32,
        (s * f(v[1])) as f32,
        (s * f(v[2])) as f32,
    ]
}

/// `0x621160`: `out[i] = a[i] + b[i]`, with `a` in `edx` and a pointer to `b` as the stack
/// argument.
pub fn vec_add(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        (f(a[0]) + f(b[0])) as f32,
        (f(a[1]) + f(b[1])) as f32,
        (f(a[2]) + f(b[2])) as f32,
    ]
}
