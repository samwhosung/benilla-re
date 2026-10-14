//! The vector leaves after the evaluators: the squared length of a vector of two and of three, and
//! the division of a vector by a scalar (`Vector2LengthSquared`, `Vector3LengthSquared` and
//! `DivideVector3`).

/// `0x454980` (`ecx` = the vector; the result in `st0`): `x·x + y·y` at 53 bits of the `f32`
/// fields at `+0` and `+4`, not stored.
#[must_use]
pub fn vec2_len2(x: f32, y: f32) -> f64 {
    let (x, y) = (f64::from(x), f64::from(y));
    x * x + y * y
}

/// `0x4549f0` (`ecx` = the vector; the result in `st0`): `(x·x + y·y) + z·z` at 53 bits of the
/// `f32` fields at `+0`, `+4` and `+8`, not stored.
#[must_use]
pub fn vec3_len2(x: f32, y: f32, z: f32) -> f64 {
    let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
    (x * x + y * y) + z * z
}

/// `0x4549c0` (`ecx` = the vector, changed in place, `s` on the stack): `1/s` at 53 bits (the `f32`
/// `1.0` at `0x7ff9d8` over `s`), then each component times it, stored as `f32`.
#[must_use]
pub fn vec_div_scalar(v: [f32; 3], s: f32) -> [f32; 3] {
    let recip = 1.0_f64 / f64::from(s);
    v.map(|c| (f64::from(c) * recip) as f32)
}
