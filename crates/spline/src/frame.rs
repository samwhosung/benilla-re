//! The segment functions that run on the curve object: `this+0x28` is the mode (0 is linear, anything
//! else spline), `this+0x10` the control points, and the frame builders write a tangent, a normal
//! and a binormal at `+0`, `+0x10` and `+0x20` (0x453b90 writes them at `+0`, `+0x10`, `+0x20` as
//! three vectors of four words). Each function is the arithmetic of the path it names; the
//! kernels it calls are in `eval`.

/// `0x4541b0`, linear mode (`ecx` = the curve, the stack arguments the segment, `t` and the
/// output): `P[seg+1] + (P[seg+2] − P[seg+1])·t` for `pa = P[seg+1]` and `pb = P[seg+2]`. The three
/// components round differently: x stores `dx·t` as `f32` before adding `pa.x`, y keeps `dy·t` in the
/// register, and z stores `dz` as `f32` before multiplying. Every result is stored as `f32`.
#[must_use]
pub fn linear_pos(pa: [f32; 3], pb: [f32; 3], t: f32) -> [f32; 3] {
    let tf = f64::from(t);
    let dx = f64::from(pb[0]) - f64::from(pa[0]);
    let dxt = (dx * tf) as f32;
    let out_x = (f64::from(dxt) + f64::from(pa[0])) as f32;
    let dy = f64::from(pb[1]) - f64::from(pa[1]);
    let out_y = (dy * tf + f64::from(pa[1])) as f32;
    let dz = (f64::from(pb[2]) - f64::from(pa[2])) as f32;
    let out_z = (f64::from(dz) * tf + f64::from(pa[2])) as f32;
    [out_x, out_y, out_z]
}

/// `0x454320`, linear mode (`ecx` = the curve, the segment on the stack; the result in `st0`): the
/// distance from `pa = P[seg+1]` to `pb = P[seg+2]` as `√((dz² + dy²) + dx²)` of the 53-bit differences.
/// The spline mode is `arc_length`.
#[must_use]
pub fn seg_length_linear(pa: [f32; 3], pb: [f32; 3]) -> f64 {
    let dx = f64::from(pb[0]) - f64::from(pa[0]);
    let dy = f64::from(pb[1]) - f64::from(pa[1]);
    let dz = f64::from(pb[2]) - f64::from(pa[2]);
    ((dz * dz + dy * dy) + dx * dx).sqrt()
}

/// The frame `0x453b90` builds from the derivative `der` its kernel `0x453cd0` evaluates (`eps`
/// is the `f32` at `0x8029d0`, 0.01, and `one` the `f32` at `0x7ff9d8`): `[T, pad, N, pad, B, pad]`
/// as three vectors of four words.
///
/// `T` is `der/len` for `len = √((dy² + dz²) + dx²)` when `|len| > eps` (strictly), and zero
/// otherwise. `N = (−T.y, T.x, 0)` divided by its length `√(x² + y²)` with no zero test, the
/// quotient kept in the register for the cross product while the field stores `f32`. `B = T × N`
/// as `(−(N.y·T.z), N.x·T.z, N.y·T.x − N.x·T.y)` from those register values and the stored `T`.
#[must_use]
pub fn frenet_frame_from_derivative(der: [f32; 3], eps: f32, one: f32) -> [f32; 12] {
    let mut f = [0.0f32; 12];
    let (dxf, dyf, dzf) = (f64::from(der[0]), f64::from(der[1]), f64::from(der[2]));
    let len = ((dyf * dyf + dzf * dzf) + dxf * dxf).sqrt();
    if len.abs() > f64::from(eps) {
        let inv = f64::from(one) / len;
        f[0] = (dxf * inv) as f32;
        f[1] = (dyf * inv) as f32;
        f[2] = (dzf * inv) as f32;
    }
    let (tx, ty, tz) = (f[0], f[1], f[2]);
    let nxf = f64::from(-ty);
    let nyf = f64::from(tx);
    let inv2 = f64::from(one) / (nxf * nxf + nyf * nyf).sqrt();
    let nx = nxf * inv2;
    let ny = nyf * inv2;
    f[4] = nx as f32;
    f[5] = ny as f32;
    let (txf, tyf, tzf) = (f64::from(tx), f64::from(ty), f64::from(tz));
    f[8] = (-(ny * tzf)) as f32;
    f[9] = (nx * tzf) as f32;
    f[10] = (ny * txf - nx * tyf) as f32;
    f
}

/// The tangent of `0x454580` in linear mode (`eps2` is the `f32` at `0x8029d4`, `2^-22`, and `one`
/// the `f32` at `0x7ff9d8`): `pb − pa` with each difference stored as `f32`, divided by the
/// length when `|len| > eps2`. The length is `√((x² + y²) + z²)` at 53 bits, stored as `f32`
/// before the reciprocal `1/len` is taken (so it divides by the narrowed length); the comparison
/// uses the unnarrowed one. A shorter difference is returned as it is.
#[must_use]
pub fn frame_tangent_linear(pa: [f32; 3], pb: [f32; 3], eps2: f32, one: f32) -> [f32; 3] {
    let diff = [0, 1, 2].map(|i| (f64::from(pb[i]) - f64::from(pa[i])) as f32);
    let [dxf, dyf, dzf] = diff.map(f64::from);
    let len = ((dxf * dxf + dyf * dyf) + dzf * dzf).sqrt();
    if len.abs() > f64::from(eps2) {
        let recip = f64::from(one) / f64::from(len as f32);
        [
            (dxf * recip) as f32,
            (dyf * recip) as f32,
            (dzf * recip) as f32,
        ]
    } else {
        diff
    }
}

/// The frame `0x454580` builds from the tangent `tan`: `T = tan`, `N = (−T.y, T.x, 0)` divided by
/// its length `√(x² + y²)` when that is above `eps2` (stored as `f32`; a shorter `N` is kept), and
/// `B = T × N` as `(−(N.y·T.z), N.x·T.z, T.x·N.y − N.x·T.y)` from the stored values, laid out as
/// `[T, pad, N, pad, B, pad]` in words of four.
#[must_use]
pub fn frame_finish(tan: [f32; 3], eps2: f32, one: f32) -> [f32; 12] {
    let mut f = [0.0f32; 12];
    let [tx, ty, tz] = tan;
    f[0] = tx;
    f[1] = ty;
    f[2] = tz;
    let nx0 = -ty;
    let ny0 = tx;
    let (nxf0, nyf0) = (f64::from(nx0), f64::from(ny0));
    let n2 = (nxf0 * nxf0 + nyf0 * nyf0).sqrt();
    let (nx, ny) = if n2.abs() > f64::from(eps2) {
        let inv = f64::from(one) / n2;
        ((nxf0 * inv) as f32, (nyf0 * inv) as f32)
    } else {
        (nx0, ny0)
    };
    f[4] = nx;
    f[5] = ny;
    let (nxf, nyf) = (f64::from(nx), f64::from(ny));
    let (txf, tyf, tzf) = (f64::from(tx), f64::from(ty), f64::from(tz));
    f[8] = (-(nyf * tzf)) as f32;
    f[9] = (nxf * tzf) as f32;
    f[10] = (txf * nyf - nxf * tyf) as f32;
    f
}

/// The tangent of `0x454580` in spline mode, given the derivative `deriv_raw` of `0x454250` (the
/// degree-2 evaluator over the derivative basis at `0xb05dd8`) and the linear tangent `lin`.
/// The derivative is divided by its length `√((x² + y²) + z²)` when the length is not below `eps2_deriv`
/// (the `f32` at `0x8029d0`, 0.01; the unnormalized derivative is `None` below, where the
/// function leaves the tangent unwritten). The result is the normalized derivative when
/// `dot = (d.z·l.z + d.y·l.y) + l.x·d.x` is not below 0.5 (a NaN counts as not below), and `lin`
/// otherwise.
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn frame_spline_tangent(
    deriv_raw: [f32; 3],
    lin: [f32; 3],
    eps2_deriv: f32,
) -> Option<[f32; 3]> {
    let [dx, dy, dz] = deriv_raw.map(f64::from);
    let len = ((dx * dx + dy * dy) + dz * dz).sqrt();
    if len < f64::from(eps2_deriv) {
        return None;
    }
    let inv = 1.0_f64 / len;
    let deriv = [(dx * inv) as f32, (dy * inv) as f32, (dz * inv) as f32];
    let dot = (f64::from(deriv[2]) * f64::from(lin[2]) + f64::from(deriv[1]) * f64::from(lin[1]))
        + f64::from(lin[0]) * f64::from(deriv[0]);
    Some(if !(dot < 0.5) { deriv } else { lin })
}
