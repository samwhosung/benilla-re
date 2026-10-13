//! The box of a volume query and the ground normal: the box `CMovement::IsVolumeObstructed`
//! (`0x631670`, `systems/collision/contact.md#obstruction-test`) queries, the six planes
//! `CMovement::RefreshGroundNormal` (`0x6332a0`,
//! `systems/collision/resolve.md#ground-normal-refresh`) builds, and the averaged normal of
//! `AverageGroundNormal` (`0x637140`, `systems/collision/walking.md#averaged-ground-normal`).

/// `[0x7ffd74]`: 0.0.
pub const ZERO: f32 = 0.0;

/// `[0x80e028]`: 0.017452406 (`0x3c8ef859`), cos 89°.
pub const SIN_1DEG: f32 = f32::from_bits(0x3c8e_f859);

/// `[0x7ff9d8]`: 1.0.
pub const ONE: f32 = f32::from_bits(0x3f80_0000);

/// `0x631685`: the box `(−r, −r, 0)` to `(r, r, H)` stored at `0xc4e5a0` and moved by the position,
/// each corner component stored as `f32`.
pub fn query_aabb(pos: [f32; 3], xy_radius: f32, z_extent: f32) -> ([f32; 3], [f32; 3]) {
    let neg_r = (-f64::from(xy_radius)) as f32;
    let pos_r = xy_radius;
    let min = [
        (f64::from(neg_r) + f64::from(pos[0])) as f32,
        (f64::from(neg_r) + f64::from(pos[1])) as f32,
        (f64::from(ZERO) + f64::from(pos[2])) as f32,
    ];
    let max = [
        (f64::from(pos_r) + f64::from(pos[0])) as f32,
        (f64::from(pos_r) + f64::from(pos[1])) as f32,
        (f64::from(z_extent) + f64::from(pos[2])) as f32,
    ];
    (min, max)
}

/// `0x6332db`: the six planes' `d`: `−x − r`, `x − r`, `−y − r`, `y − r`, `−z − h`, `z`, each
/// stored as `f32`.
pub fn capsule_kdop_dists(c: [f32; 3], radius: f32, height: f32) -> [f32; 6] {
    let x = f64::from(c[0]);
    let y = f64::from(c[1]);
    let z = f64::from(c[2]);
    let r = f64::from(radius);
    let h = f64::from(height);
    [
        ((-x) - r) as f32,
        (x - r) as f32,
        ((-y) - r) as f32,
        (y - r) as f32,
        ((-z) - h) as f32,
        z as f32,
    ]
}

/// `0x6371a4`: a face counts when its normal's z is above cos 89°, or NaN (`test ah,0x41; jnp`).
pub fn walkable_normal_accepts(nz: f32) -> bool {
    let n = f64::from(nz);

    n.is_nan() || n > f64::from(SIN_1DEG)
}

/// One step of the sum: a float add, stored.
pub fn normal_sum_step(prev: f32, term: f32) -> f32 {
    (f64::from(prev) + f64::from(term)) as f32
}

/// The mean `sum·(1/k)` and `|mean|² = (z·z₃₂ + y₃₂²) + x²`: x kept in the register, y stored, z
/// stored by `fst` (`0x637271`) and kept. Returns the square and the mean as the code holds it.
pub fn mean_and_mag2(sum: [f32; 3], count: i32) -> (f64, f64, f32, f32) {
    let inv = f64::from(ONE) / f64::from(count);

    let mx = f64::from(sum[0]) * inv;

    let my = (f64::from(sum[1]) * inv) as f32;
    let mz_f32 = (f64::from(sum[2]) * inv) as f32;
    let mz_live = f64::from(sum[2]) * inv;

    let mz2 = mz_live * f64::from(mz_f32);

    let my2 = f64::from(my) * f64::from(my);

    let mag2 = (mz2 + my2) + mx * mx;
    (mag2, mx, my, mz_f32)
}

/// `0x637287`: `1/|mean|` stored by `fst` and kept; out.x uses the register's reciprocal and mean,
/// out.y and out.z the stored ones.
pub fn renormalize(mag2: f64, mx: f64, my: f32, mz: f32) -> [f32; 3] {
    let inv_len_live = f64::from(ONE) / mag2.sqrt();

    let inv_len_f32 = inv_len_live as f32;
    [
        (inv_len_live * mx) as f32,
        (f64::from(my) * f64::from(inv_len_f32)) as f32,
        (f64::from(mz) * f64::from(inv_len_f32)) as f32,
    ]
}
