//! The two constructors of the 13-`f32` plane record the collision facets use: a normal and a
//! plane distance `[0..4]`, then three points `[4..13]`.

/// `0x7bf860` (`ecx` = the record, the stack arguments the pointers to the points `p0`, `p1` and
/// `p2`): the points are copied to `[4..13]` and the plane is the normal of `(p1 − p0) × (p2 − p0)`
/// made a unit vector, with `d = −(n · p0)`. The edges `e1x` and `e1y` are stored as `f32`
/// and `e1z`, `e2` stay in the register; the three cross components are stored as `f32`, and the
/// length is `√((nx² + ny²) + nz²)` of those stored values. `1/length` stays in the register and
/// multiplies each stored component, and those register products give `d` as `−((nz·p0z + ny·p0y) +
/// nx·p0x)`. Each output is stored as `f32`. There is no zero-length test.
#[must_use]
pub fn plane_from_3pts(p0: &[f32; 3], p1: &[f32; 3], p2: &[f32; 3]) -> [f32; 13] {
    let f = f64::from;
    let e1x = (f(p1[0]) - f(p0[0])) as f32;
    let e1y = (f(p1[1]) - f(p0[1])) as f32;
    let e1z = f(p1[2]) - f(p0[2]);
    let e2x = f(p2[0]) - f(p0[0]);
    let e2y = f(p2[1]) - f(p0[1]);
    let e2z = f(p2[2]) - f(p0[2]);
    let nx = (f(e1y) * e2z - e1z * e2y) as f32;
    let ny = (e1z * e2x - f(e1x) * e2z) as f32;
    let nz = (f(e1x) * e2y - f(e1y) * e2x) as f32;
    let sum_squares = (f(nx) * f(nx) + f(ny) * f(ny)) + f(nz) * f(nz);
    let inv_len = 1.0 / sum_squares.sqrt();
    let (ux, uy, uz) = (inv_len * f(nx), inv_len * f(ny), inv_len * f(nz));
    let d = (-((uz * f(p0[2]) + uy * f(p0[1])) + ux * f(p0[0]))) as f32;
    [
        ux as f32, uy as f32, uz as f32, d, p0[0], p0[1], p0[2], p1[0], p1[1], p1[2], p2[0], p2[1],
        p2[2],
    ]
}

/// `0x7bf7b0` (`ecx` = the record, `s` on the stack): the record `[0, 0, 1, (s + s)·0 − s, s, s,
/// s, s, s, s, s, s, s]`. The fourth entry is `−s` for a finite `s` and NaN for an infinite one,
/// since the product with zero is evaluated; the three points are `s` in every coordinate.
#[must_use]
pub fn scalar_bcast_struct13(s: f32) -> [f32; 13] {
    let f = f64::from;
    let d = ((f(s) + f(s)) * 0.0 - f(s)) as f32;
    [0.0, 0.0, 1.0, d, s, s, s, s, s, s, s, s, s]
}
