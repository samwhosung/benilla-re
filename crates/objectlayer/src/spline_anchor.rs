//! The arithmetic of `AnchorSplinePath` (`0x6021c0`,
//! `systems/objectlayer/unitmovement.md#monster-moves`), which anchors a monster path at a
//! position, and of `DotVector3` (`0x602630`), which it calls.
//!
//! `__thiscall`: `ecx` = the vertices (12 bytes each), `edx` = the count, `[ebp+0xc]` = the
//! position `P`. With one vertex it compares the squared distance with eps² (`0x6021da` on); with
//! more it walks the segments: the edge and the bracket through `0x602630` (`0x602252`), the 2D
//! distance of the first vertex (`0x602324`) and of the last (`0x6025d7`) against 1.0, the 3D
//! distance of a matched vertex (`0x60237b`), the unit edge and the projections (`0x602426`), and
//! the cut point and its distance (`0x60251c`). Each function below is one of those pieces. A
//! difference kept in the register is `f64`; one stored with `fstp dword` and reloaded is `f32`.

/// `[0x80c450]`: 0.0007716049 (`0x3a4a4588`), (1/36)², the squared radius within which two points
/// are one.
pub const EPS2: f32 = f32::from_bits(0x3a4a_4588);

/// `[0x7ff9d8]`: 1.0, the reciprocal's numerator and the 2D gate.
pub const ONE: f32 = f32::from_bits(0x3f80_0000);

/// `[0x7ffd74]`: 0.0, the sign test on the brackets.
pub const ZERO: f32 = 0.0;

/// `DotVector3` (`0x602630`, `ecx` = `a`, `edx` = `b`): `(a.z·b.z + a.y·b.y) + a.x·b.x`, left in
/// `st0` unrounded.
pub fn dot3(a: [f32; 3], b: [f32; 3]) -> f64 {
    (f64::from(a[2]) * f64::from(b[2]) + f64::from(a[1]) * f64::from(b[1]))
        + f64::from(a[0]) * f64::from(b[0])
}

/// `0x6021da`: `a − P` per component, each stored as `f32`.
pub fn diff3(a: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) - f64::from(p[0])) as f32,
        (f64::from(a[1]) - f64::from(p[1])) as f32,
        (f64::from(a[2]) - f64::from(p[2])) as f32,
    ]
}

/// `0x602200`: `(dz² + dy²) + dx²` from the stored differences, compared with eps² at `0x602216`
/// (`test ah,5; jp`: at least eps², or NaN, jumps).
pub fn dist2_3d_a(d: [f32; 3]) -> f64 {
    (f64::from(d[2]) * f64::from(d[2]) + f64::from(d[1]) * f64::from(d[1]))
        + f64::from(d[0]) * f64::from(d[0])
}

/// `0x602324` and `0x6025d7`: `dy² + dx²` from differences kept in the register, no z, compared
/// with 1.0 (`test ah,5; jnp`: below 1.0 jumps).
pub fn dist2_2d(v: [f32; 3], p: [f32; 3]) -> f64 {
    let dx = f64::from(v[0]) - f64::from(p[0]);
    let dy = f64::from(v[1]) - f64::from(p[1]);
    dy * dy + dx * dx
}

/// `0x60237b`: `(dz² + dy²) + dx²` from differences kept in the register, compared with eps² at
/// `0x6023a1`.
pub fn dist2_3d_live(v: [f32; 3], p: [f32; 3]) -> f64 {
    let dx = f64::from(v[0]) - f64::from(p[0]);
    let dy = f64::from(v[1]) - f64::from(p[1]);
    let dz = f64::from(v[2]) - f64::from(p[2]);
    (dz * dz + dy * dy) + dx * dx
}

/// `0x602252`: the edge `B − A`, each component stored as `f32`.
pub fn segment_edge(b: [f32; 3], a: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(b[0]) - f64::from(a[0])) as f32,
        (f64::from(b[1]) - f64::from(a[1])) as f32,
        (f64::from(b[2]) - f64::from(a[2])) as f32,
    ]
}

/// `0x602295`: `c = −(e·P)` through `0x602630`, stored as `f32`.
pub fn bracket_c(e: [f32; 3], p: [f32; 3]) -> f32 {
    (-dot3(e, p)) as f32
}

/// `0x6022a5` and `0x6022b5`: `e·X + c`, stored as `f32` for `A`; for `B` the sum stays in the
/// register, and its only use is a sign test, which the stored value passes the same way.
pub fn bracket_d(e: [f32; 3], x: [f32; 3], c: f32) -> f32 {
    (dot3(e, x) + f64::from(c)) as f32
}

/// `0x602426`: `|e|²` from the edge kept in the register, except that `fst [ebp-0x30]` (`0x602437`)
/// stores `ez` and the square is `ez` in the register times `ez` stored: `(ez·ez₃₂ + ey²) + ex²`.
pub fn edge_len2(e: [f64; 3]) -> f64 {
    let ez_f32 = e[2] as f32;
    let ez2 = e[2] * f64::from(ez_f32);
    let ey2 = e[1] * e[1];
    let ex2 = e[0] * e[0];
    (ez2 + ey2) + ex2
}

/// `0x602449`: `inv = 1/√|e|²` stored as `f32`; `nx = inv·ex` and `ny = inv·ey` from the register,
/// `nz = inv·ez₃₂`, each stored as `f32`.
pub fn unit_dir(e: [f64; 3], mag2: f64) -> [f32; 3] {
    let inv_f32 = (f64::from(ONE) / mag2.sqrt()) as f32;
    let ez_f32 = e[2] as f32;
    [
        (f64::from(inv_f32) * e[0]) as f32,
        (f64::from(inv_f32) * e[1]) as f32,
        (f64::from(inv_f32) * f64::from(ez_f32)) as f32,
    ]
}

/// `0x602475`: `c = −((nx·Px + nz·Pz) + ny·Py)`, stored as `f32`.
pub fn proj_c(n: [f32; 3], p: [f32; 3]) -> f32 {
    let s = f64::from(n[0]) * f64::from(p[0]) + f64::from(n[2]) * f64::from(p[2]);
    (-(s + f64::from(n[1]) * f64::from(p[1]))) as f32
}

/// `0x60248f` and `0x6024a8`: `((nz·Xz + ny·Xy) + nx·Xx) + c`, unrounded; the caller keeps `dA` in
/// the register and stores `dB`.
pub fn proj_d(n: [f32; 3], x: [f32; 3], c: f32) -> f64 {
    let s = f64::from(n[2]) * f64::from(x[2]) + f64::from(n[1]) * f64::from(x[1]);
    (s + f64::from(n[0]) * f64::from(x[0])) + f64::from(c)
}

/// `0x60251c`: `t = dA/(dA − dB)` (`fsubr`, then `fdivp`), kept in the register.
pub fn lerp_param(da: f64, db: f64) -> f64 {
    da / (da - db)
}

/// `0x602523`: the cut point `C = A + t·(B − A)` per component, stored as `f32` into the path.
pub fn lerp_point(a: [f32; 3], b: [f32; 3], t: f64) -> [f32; 3] {
    [
        (f64::from(a[0]) + t * (f64::from(b[0]) - f64::from(a[0]))) as f32,
        (f64::from(a[1]) + t * (f64::from(b[1]) - f64::from(a[1]))) as f32,
        (f64::from(a[2]) + t * (f64::from(b[2]) - f64::from(a[2]))) as f32,
    ]
}

/// `0x60254e`: `|B − C|²`, the endpoint against the cut point, as `0x60237b` sums it; compared with
/// eps² at `0x60256f`.
pub fn dist2_bc(b: [f32; 3], c: [f32; 3]) -> f64 {
    dist2_3d_live(b, c)
}
