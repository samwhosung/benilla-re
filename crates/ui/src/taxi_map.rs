//! The taxi map's arithmetic (`systems/ui/taxiroute.md#route-search`, `systems/ui/taxi.md#map-art-and-positions`):
//! the length of a path and the normalized position of a point.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fstp dword` is an `as f32`.

/// The `f32` at `0x807668`, 99999997952: the length of a path with no run of points
/// (`fld dword [0x807668]; ret` at `0x4dbc5d`).
pub const PATH_LENGTH_NONE: f32 = f32::from_bits(0x51ba_43b7);

/// The summing loop of `TaxiPathLength` (`0x4dbbd0`, `taxiroute#route-search`), `0x4dbc68` to
/// `0x4dbccf`: the length of the run `points` of a path. The function first reaches the run through
/// the two slot arguments and a hash probe (`[0xbb4ae8]` is the mask and `[0xbb4ae0]` the buckets,
/// whose node holds the count at `+0x1c` and the points at `+0x18`, 0x24 bytes apart with the
/// `{x, y, z}` at `+0x10`); a miss returns [`PATH_LENGTH_NONE`].
///
/// An empty run is 0. Otherwise the sum adds the distance from `points[i - 1]` to `points[i]` for
/// `i` from 1 (the first pass only advances). `dx` is stored as `f32` (`fstp [ebp-0x1c]`, `0x4dbc97`)
/// and reloaded for its square; `dy` and `dz` stay at register precision. The squares are added
/// `dz^2`, then `dx^2`, then `dy^2`; `sqrt` is `fsqrt`; the running sum is stored as `f32` after
/// every step (`fstp [ebp-4]`).
pub fn taxi_path_length(points: &[[f32; 3]]) -> f32 {
    let mut sum: f32 = 0.0;
    for w in points.windows(2) {
        let from = w[0];
        let to = w[1];
        let dx = (f64::from(to[0]) - f64::from(from[0])) as f32;
        let dy = f64::from(to[1]) - f64::from(from[1]);
        let dz = f64::from(to[2]) - f64::from(from[2]);
        let dxf = f64::from(dx);
        let sumsq = (dz * dz + dxf * dxf) + dy * dy;
        let len = sumsq.sqrt();
        sum = (f64::from(sum) + len) as f32;
    }
    sum
}

/// `NormalizeTaxiPoint` (`0x4dc890`, `taxi#map-art-and-positions`): the position of `point` in the
/// map's rectangle, with the axes crossed. `coeffs` is `[x0, y0, x1, y1]` (offsets `+0`, `+4`, `+8`,
/// `+0xc`) and `point` is `[px, py]`. The binary computes `(px - y0) / (y1 - y0)` for `out[1]` and
/// `(x1 - py) / (x1 - x0)` for `out[0]`, each stored as `f32` (`fstp dword`), and stores 0.0 in
/// `out[2]`. A zero divisor gives an infinity, or NaN for 0/0.
pub fn normalize_taxi_point(coeffs: [f32; 4], point: [f32; 2]) -> [f32; 3] {
    let [x0, y0, x1, y1] = coeffs.map(f64::from);
    let [px, py] = point.map(f64::from);
    let out1 = ((px - y0) / (y1 - y0)) as f32;
    let out0 = ((x1 - py) / (x1 - x0)) as f32;
    [out0, out1, 0.0]
}
