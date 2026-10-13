//! The arithmetic of `CullDoodadDefList` (`0x683340`), `CullGroupEntities` (`0x6838f0`) and
//! `TraceWorldSegmentBody` (`0x69bff0`).

/// `0x683340` from `0x683388` and `0x6838f0` from `0x683972`: the camera plane `[0xc7bcb0]` up
/// against the sphere's centre `v` at `+0x5c`, `((C1·v.y + C2·v.z) + C0·v.x) + C3 − radius` in the
/// register, stored to `+0x78`.
pub fn camplane_sort_key(c: [f32; 4], v: [f32; 3], radius: f32) -> f32 {
    let [c0, c1, c2, c3] = c.map(f64::from);
    let [vx, vy, vz] = v.map(f64::from);
    ((((c1 * vy) + c2 * vz) + c0 * vx) + c3 - f64::from(radius)) as f32
}

/// `0x683340` from `0x683470` and `0x6838f0` from `0x683a48`: the squared distance `(dz² + dx²) +
/// dy²` of the centre from the camera `0xc7cf20`, in the register, against `[0x810178]` and
/// `[0x81017c]`. The first passes 1 to `0x710b90` unless `test ah,5; jp` jumps (`0x6834ab`), the
/// second passes 1 when `test ah,5; jnp` jumps (`0x683a83`): both pass 1 only when the distance is
/// below the threshold; equal, above and NaN pass 0.
pub fn near_audible(v: [f32; 3], cam: [f32; 3], threshold: f32) -> bool {
    let dx = f64::from(v[0]) - f64::from(cam[0]);
    let dy = f64::from(v[1]) - f64::from(cam[1]);
    let dz = f64::from(v[2]) - f64::from(cam[2]);
    (dz * dz + dx * dx) + dy * dy < f64::from(threshold)
}

/// `TraceWorldSegmentBody` (`0x69bff0`) from `0x69c06c`: the hit `A + (B − A)·t`, each component
/// rounded its own way: x's product stored and reloaded before the add, y's kept in the register,
/// and z's difference stored and reloaded before the product.
pub fn segment_lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let tf = f64::from(t);
    let dxt = ((f64::from(b[0]) - f64::from(a[0])) * tf) as f32;
    let dyt = (f64::from(b[1]) - f64::from(a[1])) * tf;
    let dz = (f64::from(b[2]) - f64::from(a[2])) as f32;
    [
        (f64::from(dxt) + f64::from(a[0])) as f32,
        (dyt + f64::from(a[1])) as f32,
        (f64::from(dz) * tf + f64::from(a[2])) as f32,
    ]
}
