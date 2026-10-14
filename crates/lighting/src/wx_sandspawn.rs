//! The arithmetic of `0x6793f0` Sand::SpawnGrain (`systems/lighting/precipitation.md#sand`): the
//! cell deltas, the position, angle, speed and velocity seeds, the sky rotation angle, the world
//! position, and the lifetime. The draws (`0x4531e0`, each masked to a float in [1, 2)), the
//! rotation matrix (`0x7bdd60`, `0x7bcae0`), the path query (`0x67c760`) and the vector leaves
//! `0x4549a0` and `0x4549f0` are calls this arithmetic does not include; `0x4549f0` runs as its
//! arithmetic.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`. In the camera sum the x
//! and y sums are stored with `fst` and the global is added to the wide sums, while the z sum is
//! stored and reloaded. `fsincos` is stood in by the host's `f64` sine and cosine.

/// `[0x7ff9d8]`, 0x3f800000.
const C_ONE: f32 = 1.0;
/// `[0x7ffa24]`, 0x3f000000.
const C_HALF: f32 = 0.5;
/// `[0x7ffd6c]`, 0x3e19999a.
const C_POS_X_SCALE: f32 = 0.15;
/// `[0x810024]`, 0x3f59999a.
const C_POS_X_BIAS: f32 = 0.85;
/// `[0x810020]`, 0x3eb2b8c3 (~ 20 deg in radians).
const C_ANG_SCALE: f32 = 0.349_065_87;
/// `[0x80ffbc]`, 0x3fc8f5c3.
const C_ANG_BIAS: f32 = 1.57;
/// `[0x80abfc]`, 0x3f2aaaab.
const C_VEL_SCALE: f32 = 0.666_666_7;
/// `[0x81001c]`, 0x41955555.
const C_VEL_BIAS: f32 = 18.666_666;
/// `[0x803568]`, 0x3e2aaaab.
const C_Z_SCALE: f32 = 0.166_666_67;
/// `[0x810018]`, 0x3f555555.
const C_Z_BIAS: f32 = 0.833_333_3;
/// `[0x8680fc]`, 0x3fe00000.
const C_WORLD_SCALE: f32 = 1.75;
/// `[0x7ffd78]`, 0x3e99999a.
const C_SPEED_SCALE: f32 = 0.3;
/// `[0x810014]`, 0x404ccccd.
const C_SPEED_BIAS: f32 = 3.2;

/// A draw masked as the exe does, `raw & 0x7fffff | 0x3f800000`, read as a float in [1, 2).
#[inline]
pub fn rng_unit_float(rng_raw: u32) -> f32 {
    f32::from_bits((rng_raw & 0x007f_ffff) | 0x3f80_0000)
}

/// `0x6793f0` from `0x6793f6` to `0x679418`: a cell span `hi - lo`, rounded at the `fstp` (three
/// axes).
pub fn sand_cell_delta(hi: f32, lo: f32) -> f32 {
    (f64::from(hi) - f64::from(lo)) as f32
}

/// The position components of `0x6793f0` from `0x679458` to `0x6794ad`.
pub struct SandPos {
    /// `[esp]`: x.
    pub out_x: f32,
    /// `[esp+4]`: y.
    pub out_y: f32,
    /// `[esp+8]`: z.
    pub out_z: f32,
}

/// `0x6793f0` from `0x679458` to `0x6794ad`: three draws give wide factors, `fa = (rng_z - 1) *
/// 0.15 + 0.85` for x and `(rng - 1) - 0.5` for y and z (from `rng_y` and `rng_x`), and each is
/// multiplied by its cell delta and rounded.
pub fn sand_pos_seed(
    rng_x: f32,
    rng_y: f32,
    rng_z: f32,
    delta_x: f32,
    delta_y: f32,
    delta_z: f32,
) -> SandPos {
    let fa =
        (f64::from(rng_z) - f64::from(C_ONE)) * f64::from(C_POS_X_SCALE) + f64::from(C_POS_X_BIAS);
    let fb = (f64::from(rng_y) - f64::from(C_ONE)) - f64::from(C_HALF);
    let fc = (f64::from(rng_x) - f64::from(C_ONE)) - f64::from(C_HALF);
    SandPos {
        out_z: (fc * f64::from(delta_z)) as f32,
        out_y: (fb * f64::from(delta_y)) as f32,
        out_x: (fa * f64::from(delta_x)) as f32,
    }
}

/// The angle and its cosine and sine from `0x6794df` to `0x679518`.
pub struct SandSinCos {
    /// `[ebp+8]`: the angle stored as f32, which feeds `fsincos`.
    pub angle: f32,
    /// `[ebp-8]`: the cosine.
    pub cos: f32,
    /// `[ebp-4]`: the sine.
    pub sin: f32,
}

/// `0x6793f0` from `0x6794df` to `0x679509`: `((rng - 1) - 0.5) * 0.34906587 - 1.57`, rounded at
/// the `fstp [ebp+8]`.
pub fn sand_angle(rng: f32) -> f32 {
    (((f64::from(rng) - f64::from(C_ONE)) - f64::from(C_HALF)) * f64::from(C_ANG_SCALE)
        - f64::from(C_ANG_BIAS)) as f32
}

/// `0x6793f0` from `0x6794df` to `0x679518`: the stored angle reloaded as f32 for `fsincos`.
pub fn sand_sincos(rng: f32) -> SandSinCos {
    let angle = sand_angle(rng);
    let a = f64::from(angle);
    SandSinCos {
        angle,
        cos: a.cos() as f32,
        sin: a.sin() as f32,
    }
}

/// `0x6793f0` from `0x67952f` to `0x679552`: the speed `((rng - 1) - 0.5) * 0.6666667 + 18.666666`,
/// rounded at the `fstp [ebp+8]`.
pub fn sand_vel_mag(rng: f32) -> f32 {
    (((f64::from(rng) - f64::from(C_ONE)) - f64::from(C_HALF)) * f64::from(C_VEL_SCALE)
        + f64::from(C_VEL_BIAS)) as f32
}

/// The horizontal velocity from `0x679557` to `0x679580`.
pub struct SandVelXy {
    /// `[ebp-0x20]`: speed times the cosine.
    pub vx: f32,
    /// `[ebp-0x1c]`: speed times the sine.
    pub vy: f32,
}

/// `0x6793f0` from `0x679557` to `0x679580`: the stored speed times the cosine and the sine, each
/// rounded.
pub fn sand_vel_xy(mag: f32, cos: f32, sin: f32) -> SandVelXy {
    SandVelXy {
        vx: (f64::from(mag) * f64::from(cos)) as f32,
        vy: (f64::from(mag) * f64::from(sin)) as f32,
    }
}

/// `0x6793f0` from `0x679580` to `0x6795a4`: `((rng - 1) - 0.5) * 0.16666667 + 0.8333333`, rounded
/// at the `fstp [ebp-0x18]`.
pub fn sand_vel_z(rng: f32) -> f32 {
    (((f64::from(rng) - f64::from(C_ONE)) - f64::from(C_HALF)) * f64::from(C_Z_SCALE)
        + f64::from(C_Z_BIAS)) as f32
}

/// `0x6793f0` from `0x6795af` to `0x6795bf`: the sky angle `[0xc6326c+0x78]` negated for the
/// rotation builder.
pub fn sand_negate(angle: f32) -> f32 {
    (-f64::from(angle)) as f32
}

/// `0x6793f0` from `0x6796b7` to `0x6796fd`: a wind anchor component `[0xc6326c+0x50]` times 1.75,
/// rounded.
pub fn sand_world_scale(v: f32) -> f32 {
    (f64::from(v) * f64::from(C_WORLD_SCALE)) as f32
}

/// The world position of `0x6793f0` from `0x679702` to `0x679742`.
pub struct SandCamGlobal {
    /// `[esi]`.
    pub out_x: f32,
    /// `[esi+4]`.
    pub out_y: f32,
    /// `[esi+8]`.
    pub out_z: f32,
}

/// `0x6793f0` from `0x679702` to `0x679742`: the scaled anchor plus the camera plus the global. The
/// x and y sums are stored with `fst` and the global is added to the wide sums; the z sum is stored
/// as f32 and the global is added to the stored value.
pub fn sand_cam_global(world: [f32; 3], cam: [f32; 3], g0: f32, g1: f32, g2: f32) -> SandCamGlobal {
    let sx = f64::from(world[0]) + f64::from(cam[0]);
    let sy = f64::from(world[1]) + f64::from(cam[1]);
    let sz_f32 = (f64::from(world[2]) + f64::from(cam[2])) as f32;
    SandCamGlobal {
        out_x: (sx + f64::from(g0)) as f32,
        out_y: (sy + f64::from(g1)) as f32,
        out_z: (f64::from(sz_f32) + f64::from(g2)) as f32,
    }
}

/// `0x6793f0` from `0x679754` to `0x679772`: `((rng - 1) - 0.5) * 0.3 + 3.2`, rounded at the `fstp
/// [ebp+8]`.
pub fn sand_speed(rng: f32) -> f32 {
    (((f64::from(rng) - f64::from(C_ONE)) - f64::from(C_HALF)) * f64::from(C_SPEED_SCALE)
        + f64::from(C_SPEED_BIAS)) as f32
}

/// `0x6793f0` from `0x67979f` to `0x6797c1`: the stored speed times a velocity component.
pub fn sand_vel_scale(speed: f32, v: f32) -> f32 {
    (f64::from(speed) * f64::from(v)) as f32
}

/// `0x6793f0` from `0x6797c6` to `0x6797e8`: the scaled velocity plus the position.
pub fn sand_vel_offset(scaled_vel: f32, pos: f32) -> f32 {
    (f64::from(scaled_vel) + f64::from(pos)) as f32
}

/// `0x6793f0` from `0x679817` to `0x679864`: `sqrt(|pos - hit|²) / sqrt(|vel|²) + param3`. The
/// three differences are stored as f32, `0x4549f0` sums the squares wide, the first root is stored
/// as f32 and the second stays wide into the divide.
pub fn sand_lifetime(pos: [f32; 3], hit: [f32; 3], vel: [f32; 3], param3: f32) -> f32 {
    let dx = (f64::from(pos[0]) - f64::from(hit[0])) as f32;
    let dy = (f64::from(pos[1]) - f64::from(hit[1])) as f32;
    let dz = (f64::from(pos[2]) - f64::from(hit[2])) as f32;
    let dot1 = dot3_pc53(dx, dy, dz);
    let d1 = (dot1.sqrt()) as f32;
    let dot2 = dot3_pc53(vel[0], vel[1], vel[2]);
    let d2 = dot2.sqrt();
    (f64::from(d1) / d2 + f64::from(param3)) as f32
}

/// The squared length of `0x4549f0`, `(a² + b²) + c²` wide.
fn dot3_pc53(a: f32, b: f32, c: f32) -> f64 {
    let (a, b, c) = (f64::from(a), f64::from(b), f64::from(c));
    (a * a + b * b) + c * c
}
