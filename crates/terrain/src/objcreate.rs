//! The arithmetic of `CWorld::CreateLightObjectB` (`0x671010`) and `CWorld::CreateLightObjectA`
//! (`0x671380`), `CullInteriorMapObject` (`0x685a90`) and `BuildConvexPlanes` (`0x686640`).

/// `[0x7ffe58]`: 255.0.
pub const COLOR_SCALE: f32 = 255.0;
/// `[0x7ffa24]`: 0.5.
pub const COLOR_BIAS: f32 = 0.5;
/// `[0x80c4f4]`: 0.0001 (`0x38d1b717`), the least squared length `0x685a90` normalizes.
pub const NORMALIZE_EPS2: f32 = f32::from_bits(0x38d1_b717);

/// `0x671010` from `0x671072` to `0x6710b7`, and the same at `0x6713e2` in `0x671380`: one colour
/// component for `__ftol`. At or below 0.0 it is 0.0 (`test ah,0x41; jp`), at or above 1.0 it is
/// 255.0 (`test ah,1; jne`), else `v·255 + 0.5` in the register; NaN takes the product.
pub fn color_clamp_scale(v: f32) -> f64 {
    let vd = f64::from(v);
    if vd <= 0.0 {
        0.0
    } else if vd >= 1.0 {
        f64::from(COLOR_SCALE)
    } else {
        vd * f64::from(COLOR_SCALE) + f64::from(COLOR_BIAS)
    }
}

/// `0x671010` from `0x6712fc` to `0x671319`, and the same from `0x67165c` in `0x671380`: the
/// position's length `√((x·x + y·y) + z·z)` with the three loads kept in the register, stored to
/// `[esi+0x30]`.
pub fn pos_magnitude(pos: [f32; 3]) -> f32 {
    let [x, y, z] = pos.map(f64::from);
    ((x * x + y * y) + z * z).sqrt() as f32
}

/// `CullInteriorMapObject` (`0x685a90`) at `0x685aa0`, `0x685ab8` and `0x685ad4`: a camera
/// component negated (`fchs`) and stored.
pub fn corner_ray_negate(g: f32) -> f32 {
    (-f64::from(g)) as f32
}

/// `0x685a90` from `0x685c5b` to `0x685c97`: the target `0xca8018` less the base `0xca7edc`, x and
/// y stored to `[ebp-0xc]` and `[ebp-0x8]`, z stored with `fst` and kept in the register.
pub fn dir_subtract(target: [f32; 3], base: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (f64::from(target[i]) - f64::from(base[i])) as f32)
}

/// `0x685c9a` to `0x685cab`: the squared length `(dz·f32(dz) + dx·dx) + dy·dy`, where the first `dz`
/// is the unrounded value `fst` left (`dz_live`) and the rest are the stored floats.
pub fn dir_len2(d: [f32; 3], dz_live: f64) -> f64 {
    let dz2 = dz_live * f64::from(d[2]);
    let dx2 = f64::from(d[0]) * f64::from(d[0]);
    let dy2 = f64::from(d[1]) * f64::from(d[1]);
    (dz2 + dx2) + dy2
}

/// `0x685cad`: `fcom [0x80c4f4]; test ah,0x41; jne` skips the scale at or below 0.0001 and on NaN.
pub fn dir_normalizes(len2: f64) -> bool {
    len2 > f64::from(NORMALIZE_EPS2)
}

/// `0x685cad` to `0x685cd7`: when `dir_normalizes`, each stored component times `1/√len2`
/// (`fsqrt; fdivr [0x7ff9d8]`, kept in the register), stored back; else the direction unchanged.
pub fn dir_normalize(d: [f32; 3], len2: f64) -> [f32; 3] {
    if !dir_normalizes(len2) {
        return d;
    }
    let inv = 1.0_f64 / len2.sqrt();
    d.map(|c| (f64::from(c) * inv) as f32)
}

/// `0x685d03`: the unrounded `DotVector3` (`0x602630`) of the direction and the base negated and
/// stored to `[0xca7efc]`.
pub fn plane_offset_negate(dot_live: f64) -> f32 {
    (-dot_live) as f32
}

/// `BuildConvexPlanes` (`0x686640`) at `0x6866bb`, `0x686714`, `0x686775` and `0x6867c6`: a side
/// plane's `d`, the unrounded `DotVector3` (`0x602630`) of its normal and a corner negated.
pub fn plane_d_negate(dot_live: f64) -> f32 {
    (-dot_live) as f32
}

/// `0x686640` from `0x6867ce` to `0x686816`: the fifth plane, the fourth's normal `[edi]` negated
/// (each stored), and `d = −((n.z·vp.z + n.y·vp.y) + n.x·vp.x)` with the point at `this+0x78`.
pub fn plane5_from_plane4(n4: [f32; 3], vp: [f32; 3]) -> ([f32; 3], f32) {
    let n5 = n4.map(|c| (-f64::from(c)) as f32);
    let pz = f64::from(n5[2]) * f64::from(vp[2]);
    let py = f64::from(n5[1]) * f64::from(vp[1]);
    let px = f64::from(n5[0]) * f64::from(vp[0]);
    let d5 = (-((pz + py) + px)) as f32;
    (n5, d5)
}
