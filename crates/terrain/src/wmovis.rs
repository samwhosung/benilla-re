//! The arithmetic of `CullExteriorMapObjects` (`0x6856c0`), `WalkPortals` (`0x6b41c0`) and
//! `InitTerrainTwoPi8` (`0x699210`, `systems/terrain/statics.md#static-initializers`).

/// `[0x7ff9d8]`: 1.0.
pub const WMO_ONE: f32 = 1.0;
/// `[0x80c4f4]`: 0.0001 (`0x38d1b717`).
pub const WMO_NORMALIZE_EPS2: f32 = f32::from_bits(0x38d1_b717);
/// `[0x7ffd74]`: 0.0.
pub const PORTAL_ZERO: f32 = 0.0;

/// `CullExteriorMapObjects` (`0x6856c0`) at `0x685864`, `0x685884` and `0x6858a1`: a camera component
/// negated and stored.
pub fn wmo_corner_ray_negate(g: f32) -> f32 {
    (-f64::from(g)) as f32
}

/// `0x6856c0` from `0x685974`: the target `0xca8018` less the base `0xca7edc`, each stored as an
/// argument to `0x4549a0`, which copies them to `[ebp-0xc]` up.
pub fn wmo_dir_subtract(target: [f32; 3], base: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (f64::from(target[i]) - f64::from(base[i])) as f32)
}

/// `0x6856c0` from `0x6859ae`: `(x² + y²) + z²` of the stored direction, in the register.
pub fn wmo_dir_len2(d: [f32; 3]) -> f64 {
    let [x, y, z] = d.map(f64::from);
    (x * x + y * y) + z * z
}

/// `0x6856c0` at `0x6859c4`: `fcom [0x80c4f4]; test ah,0x41; jne` scales only above 0.0001.
pub fn wmo_dir_normalizes(len2: f64) -> bool {
    len2 > f64::from(WMO_NORMALIZE_EPS2)
}

/// `0x6856c0` from `0x6859d1` to `0x6859ee`: when `wmo_dir_normalizes`, each component times
/// `1/√len2` kept in the register, stored back.
pub fn wmo_dir_normalize(d: [f32; 3], len2: f64) -> [f32; 3] {
    if !wmo_dir_normalizes(len2) {
        return d;
    }
    let inv = f64::from(WMO_ONE) / len2.sqrt();
    d.map(|c| (f64::from(c) * inv) as f32)
}

/// `0x6856c0` at `0x685a1a`: the unrounded `DotVector3` (`0x602630`) negated to `[0xca7efc]`.
pub fn wmo_plane_offset_negate(dot_live: f64) -> f32 {
    (-dot_live) as f32
}

/// `WalkPortals` (`0x6b41c0`) from `0x6b43d2`: `((pz·n.z + py·n.y) + px·n.x) + d` of the point
/// `0xca7edc` and the portal plane `[ebx+0x4]`, negated (`fchs`) when `flip`; the client compares it
/// unrounded with 0.0, and this is that value stored as `f32`.
pub fn portal_plane_dot(pt: [f32; 3], plane: [f32; 3], plane_d: f32, flip: bool) -> f32 {
    let s = ((f64::from(pt[2]) * f64::from(plane[2]) + f64::from(pt[1]) * f64::from(plane[1]))
        + f64::from(pt[0]) * f64::from(plane[0]))
        + f64::from(plane_d);
    (if flip { -s } else { s }) as f32
}

/// `0x6b41c0` at `0x6b44b1` and `0x6b44ca`: `|c1 − c0|` of the rectangle's sides, compared with
/// 0.001; this is that value stored as `f32`.
pub fn portal_rect_extent(c1: f32, c0: f32) -> f32 {
    (f64::from(c1) - f64::from(c0)).abs() as f32
}

/// `0x6b41c0` from `0x6b451e` and from `0x6b45ba`: each corner `(c + 1)·0.5`, the first two sums kept
/// in the register and the last two stored first.
pub fn portal_ndc_to_screen(coords: [f32; 4]) -> [f32; 4] {
    let one = f64::from(WMO_ONE);
    let half = 0.5f64;
    let live = |c: f32| ((f64::from(c) + one) * half) as f32;
    let stored = |c: f32| (f64::from((f64::from(c) + one) as f32) * half) as f32;
    [
        live(coords[0]),
        live(coords[1]),
        stored(coords[2]),
        stored(coords[3]),
    ]
}

/// `InitTerrainTwoPi8` (`0x699210`): `[0x810618]` added to itself (`fadd st(0),st`), stored to
/// `[0xca0538]`.
pub fn wmo_group_double(x: f32) -> f32 {
    (f64::from(x) + f64::from(x)) as f32
}
