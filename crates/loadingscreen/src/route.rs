//! The taxi-route ship animation geometry of `LoadingScreen.cpp`.

/// Ship-sprite half-extents (`.rdata`): half-height 0.0265 (`0x7ffda8`), half-width 0.02 (`0x7ffdac`).
const HALF_H: f32 = f32::from_bits(0x3cd9_1687); // 0.0265
const HALF_W: f32 = f32::from_bits(0x3ca3_d70a); // 0.02
/// Taxi-route tessellation density (`0x7ffd80`): 66.66667 segments per unit arc length.
const SEG_DENSITY: f32 = f32::from_bits(0x4285_5556);

/// The segment count of the taxi-route tessellator `0x4075a0`.
///
/// The route's `arclen` comes from the spline arc-length builder `0x453360`. The ribbon is split into
/// `segs = max(trunc(arclen * 66.66667), 1)` segments, and the vertex count is `segs * 4 + 12`. The
/// `fld`, `fmul` rounds to `f64`; `__ftol` truncates toward zero.
pub fn route_tessellate_segs(arclen: f32) -> i32 {
    let prod = arclen as f64 * SEG_DENSITY as f64;
    benilla_re_cmath::x87::ftol(prod).max(1)
}

/// The endpoint quads of `0x407900` (`__fastcall (a ecx, b edx)`).
///
/// Fills the first eight `DYNAMICELEMENTVERT`s, two marker quads at `a` and `b`, into the route
/// vertex buffer (`0x8828e8`). Each quad is centred on `c = (p[0], p[1])` with corners
/// `x` in `{c.x - 0.02, c.x + 0.02}`, `y` in `{c.y - 0.0265, c.y + 0.0265}` and `z = 0`. The UVs are
/// constants: A takes the sprite's right half, `u` in `{0.5, 1.0}`, and B the left half, `u` in
/// `{0.0, 0.5}`; `v` is 0.5 at the bottom and 0.0 at the top. Returns each vertex's `(x, y, z, u, v)`;
/// the sixth dword, the colour `0x8828a4`, is copied unchanged. The subtraction and addition are `f64`,
/// the store `f32`.
pub fn route_endpoint_quads(a: [f32; 2], b: [f32; 2]) -> [[f32; 5]; 8] {
    let mk = |c: [f32; 2], u_lo: f32, u_hi: f32| -> [[f32; 5]; 4] {
        let xl = (c[0] as f64 - HALF_W as f64) as f32;
        let xr = (c[0] as f64 + HALF_W as f64) as f32;
        let yb = (c[1] as f64 - HALF_H as f64) as f32; // c.y − 0.0265 (v=0.5)
        let yt = (c[1] as f64 + HALF_H as f64) as f32; // c.y + 0.0265 (v=0.0)
        [
            [xl, yb, 0.0, u_lo, 0.5],
            [xr, yb, 0.0, u_hi, 0.5],
            [xl, yt, 0.0, u_lo, 0.0],
            [xr, yt, 0.0, u_hi, 0.0],
        ]
    };
    let qa = mk(a, 0.5, 1.0); // endpoint A: right half
    let qb = mk(b, 0.0, 0.5); // endpoint B: left half
    [qa[0], qa[1], qa[2], qa[3], qb[0], qb[1], qb[2], qb[3]]
}
