//! The arithmetic of `SetWorldObjectBounds` (`0x6717d0`), which sets a world object's transform and
//! box, and of `CWorld::PrepareAreaOfInterest` (`0x672730`), which recomputes the windows about the
//! camera.
//!
//! `0x6717d0` keeps a length in the register after an `fst dword` (no pop): the test after it uses
//! the 53-bit value, and the reciprocal reloads the stored `f32`.

/// `[0x801360]`: 0.001 (`0x3a83126f`), the least squared move.
pub const MOVE_EPS2: f32 = f32::from_bits(0x3a83_126f);

/// `[0x8029d4]`: 2^-22 (`0x34800000`), the least `|len − 1|` that renormalizes.
pub const RENORM_EPS: f32 = f32::from_bits(0x3480_0000);

/// `[0x7ff9d8]`: 1.0.
pub const ONE: f32 = 1.0;

/// `[0x7ffa24]`: 0.5.
pub const HALF: f32 = 0.5;

/// `0x6717da` to `0x671802`: `new − old` per component, stored z, y, x; `new` is the argument's
/// `+0x30`, `old` the object's `+0x24`.
pub fn camera_pos_delta(new_pos: [f32; 3], old_pos: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (f64::from(new_pos[i]) - f64::from(old_pos[i])) as f32)
}

/// `0x67180a` to `0x671820`: `|(dz² + dy²) + dx²|` of the stored deltas, kept at 53 bits.
pub fn camera_move_dist2(d: [f32; 3]) -> f64 {
    let dz2 = f64::from(d[2]) * f64::from(d[2]);
    let dy2 = f64::from(d[1]) * f64::from(d[1]);
    let s1 = dy2 + dz2;
    let dx2 = f64::from(d[0]) * f64::from(d[0]);
    (s1 + dx2).abs()
}

/// `0x671820` to `0x671836`: `fcomp [0x801360]; test ah,5; jp` sets `eax = 1` when the squared move
/// is below 0.001; equal, above and NaN give 0.
pub fn camera_move_flag(dist2: f64) -> bool {
    let eps = f64::from(MOVE_EPS2);
    dist2 < eps
}

/// `0x671853` to `0x67186b`: the length of the object's `+0x0` vector, `√((x² + y²) + z²)` at 53
/// bits.
pub fn camera_fwd_mag(fwd: [f32; 3]) -> f64 {
    let x = f64::from(fwd[0]);
    let y = f64::from(fwd[1]);
    let z = f64::from(fwd[2]);
    ((x * x) + (y * y) + (z * z)).sqrt()
}

/// `0x671873` to `0x671889`: `fst dword [+0x30]`, then `|len − 1|` of the register's 53-bit length
/// against 2^-22, `test ah,5; jnp 0x671901`. Below skips the renormalization; equal, above and NaN
/// renormalize, returning true.
pub fn camera_fwd_renorm_decision(mag: f64) -> bool {
    let diff = (mag - f64::from(ONE)).abs();
    let eps = f64::from(RENORM_EPS);
    !matches!(diff.partial_cmp(&eps), Some(std::cmp::Ordering::Less))
}

/// The `f32` the `fst dword` at `0x671873` stores to `+0x30`.
pub fn camera_fwd_mag_stored(mag: f64) -> f32 {
    mag as f32
}

/// `0x6718b6` to `0x6718bf`: `f32(1 / len)`, the divisor the stored `+0x30` reloaded; the scale
/// passed to `ScaleMatrix3` (`0x672d80`).
pub fn camera_inv_mag(mag_f32: f32) -> f32 {
    (f64::from(ONE) / f64::from(mag_f32)) as f32
}

/// `0x671986` to `0x6719ae`: three components times 0.5, stored z, y, x.
pub fn camera_right_half(right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (f64::from(right[i]) * f64::from(HALF)) as f32)
}

/// `0x6719cc` to `0x6719f5`: `|v(+0x50) − v(+0x5c)|` stored at `+0x68`; the differences loaded x,
/// y, z, so the sum is `(dz² + dy²) + dx²`.
pub fn camera_up_renorm_mag(up: [f32; 3], base: [f32; 3]) -> f32 {
    let dx = f64::from(up[0]) - f64::from(base[0]);
    let dy = f64::from(up[1]) - f64::from(base[1]);
    let dz = f64::from(up[2]) - f64::from(base[2]);
    (((dz * dz) + (dy * dy)) + (dx * dx)).sqrt() as f32
}

/// `[0x80fee4]`: the chunk edge, 33.333332 (`0x42055555`).
pub const BOX_STEP: f32 = f32::from_bits(0x4205_5555);
/// `[0x80fee0]`: 150.0.
pub const BOX_150: f32 = 150.0;
/// `[0x7ffab4]`: the map's half extent, 17066.666 (`0x46855555`).
pub const MAP_CENTER: f32 = f32::from_bits(0x4685_5555);
/// `[0x80fe84]`: 0.03 (`0x3cf5c28f`), units to chunks.
pub const CHUNK_SCALE: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x867914]`: 0.5, subtracted before `fistp`.
pub const CHUNK_BIAS: f32 = 0.5;

/// `0x672736` to `0x672773`: the far clip box about the centre on x and y, `[cx − far, cy − far,
/// far + cx, far + cy]`, stored to `0xc631b8`, `0xc631b4`, `0xc631c0`, `0xc631bc`.
pub fn window_farclip_box(center: [f32; 3], farclip: f32) -> [f32; 4] {
    let far = f64::from(farclip);
    [
        (f64::from(center[0]) - far) as f32,
        (f64::from(center[1]) - far) as f32,
        (far + f64::from(center[0])) as f32,
        (far + f64::from(center[1])) as f32,
    ]
}

/// `0x672773` to `0x6727b5`: that box widened by 33.333 each way from its stored values, to
/// `0xc631a4`, `0xc631a0`, `0xc631ac`, `0xc631a8`.
pub fn window_farclip_box_step(box4: [f32; 4]) -> [f32; 4] {
    let s = f64::from(BOX_STEP);
    [
        (f64::from(box4[0]) - s) as f32,
        (f64::from(box4[1]) - s) as f32,
        (f64::from(box4[2]) + s) as f32,
        (f64::from(box4[3]) + s) as f32,
    ]
}

/// `0x6727bb` to `0x67282b`: the box of ±150 about the centre, the low corner then the high.
pub fn window_box_150(center: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let h = f64::from(BOX_150);
    let lo = std::array::from_fn(|i| (f64::from(center[i]) - h) as f32);
    let hi = std::array::from_fn(|i| (f64::from(center[i]) + h) as f32);
    (lo, hi)
}

/// `0x67282e` to `0x672895`: the far clip box on three axes, the far clip `[0xc7b498]` loaded once
/// and kept in the register; `c − far` and `far + c`.
pub fn window_farclip_box_live(center: [f32; 3], farclip: f32) -> ([f32; 3], [f32; 3]) {
    let far = f64::from(farclip);
    let lo = std::array::from_fn(|i| (f64::from(center[i]) - far) as f32);
    let hi = std::array::from_fn(|i| (far + f64::from(center[i])) as f32);
    (lo, hi)
}

/// `0x6728a1` to `0x6728e3`: the chunk indices of the centre. x: `fistp(f32(−(cx − C)·0.03) −
/// 0.5)`, the negation in the register; y: `fistp(f32(0.03·f32(−(cy − C))) − 0.5)`, the negation
/// stored at `0x6728ac`. The clamp to `[0, 0x3ff]` after it is not modelled.
pub fn window_chunk_indices(center: [f32; 3]) -> (i32, i32) {
    let neg_x = -(f64::from(center[0]) - f64::from(MAP_CENTER));
    let prod_x = (neg_x * f64::from(CHUNK_SCALE)) as f32;
    let idx_x = crate::x87::fistp(f64::from(prod_x) - f64::from(CHUNK_BIAS));
    let neg_y = (-(f64::from(center[1]) - f64::from(MAP_CENTER))) as f32;
    let prod_y = (f64::from(CHUNK_SCALE) * f64::from(neg_y)) as f32;
    let idx_y = crate::x87::fistp(f64::from(prod_y) - f64::from(CHUNK_BIAS));
    (idx_x, idx_y)
}
