//! `0x455b30`, the wrap of an angle into `[0, 2π)`.

/// `0x455b30` (`cdecl`, `a` on the stack, `ret 4`; the result in `st0`): `r = fmod(a, 2π)` through
/// `__CIfmod` (`0x73f90a`), with `2π` the `f64` at `0x803090`; then `r + 2π` (the `f32` at
/// `0x80295c`, which widens to the same value) when `r < 0` strictly, a remainder of −0 or NaN
/// being returned as it is. The sum is taken at 53 bits and left in the register.
///
/// `__CIfmod` classifies both operands with `fxam` and jumps through a table (`0x875eb0`): two
/// normal operands go to the `fprem` loop at `0x73f914`, which gives the exact remainder with the
/// sign of `a`, and a zero `a` of either sign goes to `0x743a34`, which loads +0. A NaN or
/// infinite `a` takes the error handlers (`0x743a6d`, `0x743ad6`), which this does not model.
#[must_use]
pub fn angle_wrap(a: f32) -> f64 {
    let two_pi = f64::from(f32::from_bits(0x40c9_0fdb));
    let r = if a == 0.0 { 0.0 } else { f64::from(a) % two_pi };
    if r < 0.0 {
        r + two_pi
    } else {
        r
    }
}
