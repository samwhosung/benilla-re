//! The arithmetic of `0x674c50` Rain::SpawnDrop (`systems/lighting/precipitation.md#rain-drops`):
//! the cell deltas, the position and angle seeds, the velocity, the wind tilt, the sky rotation,
//! the world position, the collision legs and the lifetime. The draws (`0x4531e0`, each masked to a
//! float in [1, 2)), the rotation matrix (`0x7bdd60`, `0x7bcae0`), the clamped lerp `0x674ba0`, the
//! path queries (`0x67c760` and its table) and the vector leaves `0x4549a0` and `0x4549f0` are
//! calls this arithmetic does not include; the two vector leaves run as their arithmetic.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`. Where the exe stores a
//! value with `fst` and goes on with the wide one, the first use takes the wide value and the later
//! uses the stored one (the wind factor and the camera sum). `fsincos`, `fsin` and `fcos` are stood
//! in by the host's `f64` functions.

/// `[0x7ff9d8]`, 0x3f800000.
const C_ONE: f32 = 1.0;
/// `[0x7ffa24]`, 0x3f000000.
const C_HALF: f32 = 0.5;
/// `[0x7ffd74]`, 0x00000000.
const C_ZERO: f32 = 0.0;
/// `[0x80ffc4]` (about 0.20943952): scales the weather value in the angle spread.
const C_ANG_W_SCALE: f32 = f32::from_bits(0x3e56_7750);
/// `[0x80ffc0]` (about 0.052359879): the base of the angle spread.
const C_ANG_W_BIAS: f32 = f32::from_bits(0x3d56_7750);
/// `[0x80ffbc]`, 0x3fc8f5c3.
const C_ANG_BIAS: f32 = 1.57;
/// `[0x80ffb8]`, 0x4117d70a.
const C_VEL_BIAS: f32 = 9.49;
/// `[0x8029d0]`, 0x3c23d70a.
const C_VEL_EPS: f32 = 0.01;
/// `[0x8038dc]`, 0xc0000000.
const C_VZ_NEG2: f32 = -2.0;
/// `[0x80306c]`, 0x40800000.
const C_VZ_FOUR: f32 = 4.0;
/// `[0x80ffb4]`, 0x41e00000.
const C_VZ_BIAS: f32 = 28.0;
/// `[0x808acc]`, 0x3fc90fdb (~ pi/2).
const C_SKY_BIAS: f32 = 1.570_796_4;
/// `[0x80ffb0]`, 0x3d638e39 (~ 1/18).
const C_SPEED_SCALE: f32 = 0.055_555_556;
/// `[0x8680f8]`, 0x3fe00000.
const C_WORLD_SCALE: f32 = 1.75;

/// A draw masked as the exe does, `raw & 0x7fffff | 0x3f800000`, read as a float in [1, 2).
#[inline]
pub fn rng_unit_float(rng_raw: u32) -> f32 {
    f32::from_bits((rng_raw & 0x007f_ffff) | 0x3f80_0000)
}

/// `0x674c50` from `0x674c5c` to `0x674c7d`: a cell span `hi - lo`, rounded at the `fstp` (three
/// axes).
pub fn rainspawn_cell_delta(hi: f32, lo: f32) -> f32 {
    (f64::from(hi) - f64::from(lo)) as f32
}

/// The three position components of `0x674c50` from `0x674ca6` to `0x674ce8`.
pub struct RainPos {
    pub out_x: f32,
    pub out_y: f32,
    pub out_z: f32,
}

/// `0x674c50` from `0x674ca6` to `0x674ce8`: two draws `(rng - 1) - 0.5` kept wide give the x and y
/// factors; the stores are `dz * 0.0` for z, `dy * fb` for y and `dx * fc` for x, each rounded.
pub fn rainspawn_pos_seed(rng1: f32, rng2: f32, dx: f32, dy: f32, dz: f32) -> RainPos {
    let fc = (f64::from(rng2) - f64::from(C_ONE)) - f64::from(C_HALF);
    let fb = (f64::from(rng1) - f64::from(C_ONE)) - f64::from(C_HALF);
    RainPos {
        out_z: (f64::from(dz) * f64::from(C_ZERO)) as f32,
        out_y: (fb * f64::from(dy)) as f32,
        out_x: (fc * f64::from(dx)) as f32,
    }
}

/// The angle and its cosine and sine from `0x674d04` to `0x674d4b`.
pub struct RainSinCos {
    pub angle: f32,
    pub cos: f32,
    pub sin: f32,
}

/// `0x674c50` from `0x674d04` to `0x674d3c`: `((rng - 1) - 0.5) * (w * 0.20943952 + 0.05235988) -
/// 1.57`, rounded at the `fstp [ebp-0x24]`, with `w` the weather value `[esi+0xd0]`.
pub fn rainspawn_angle(rng: f32, w: f32) -> f32 {
    let fc = (f64::from(rng) - f64::from(C_ONE)) - f64::from(C_HALF);
    let g = f64::from(w) * f64::from(C_ANG_W_SCALE) + f64::from(C_ANG_W_BIAS);
    (fc * g - f64::from(C_ANG_BIAS)) as f32
}

/// `0x674c50` from `0x674d04` to `0x674d4b`: the stored angle reloaded as f32 for `fsincos`.
pub fn rainspawn_sincos(rng: f32, w: f32) -> RainSinCos {
    let angle = rainspawn_angle(rng, w);
    let a = f64::from(angle);
    RainSinCos {
        angle,
        cos: a.cos() as f32,
        sin: a.sin() as f32,
    }
}

/// `0x674c50` from `0x674d62` to `0x674d8d`: the speed `(2 * ((rng - 1) - 0.5) + 9.49) * w + 0.01`,
/// the doubling a `fadd st,st`, rounded at the `fstp [ebp-4]`.
pub fn rainspawn_vel_mag(rng: f32, w: f32) -> f32 {
    let fc = (f64::from(rng) - f64::from(C_ONE)) - f64::from(C_HALF);
    let two_fc = fc + fc;
    ((two_fc + f64::from(C_VEL_BIAS)) * f64::from(w) + f64::from(C_VEL_EPS)) as f32
}

/// The horizontal velocity from `0x674d9c` to `0x674db7`.
pub struct RainVelXy {
    pub vx: f32,
    pub vy: f32,
}

/// `0x674c50` from `0x674d9c` to `0x674db7`: the stored speed times the sine and the cosine, each
/// rounded.
pub fn rainspawn_vel_xy(mag: f32, sin: f32, cos: f32) -> RainVelXy {
    RainVelXy {
        vx: (f64::from(mag) * f64::from(sin)) as f32,
        vy: (f64::from(mag) * f64::from(cos)) as f32,
    }
}

/// `0x674c50` from `0x674db7` to `0x674df6`: `(rng - 1) * w * -2 - w * 4 - 28`, rounded at the
/// `fstp [ebp-0x14]`.
pub fn rainspawn_vel_z(rng: f32, w: f32) -> f32 {
    let a = (f64::from(rng) - f64::from(C_ONE)) * f64::from(w) * f64::from(C_VZ_NEG2);
    let b = f64::from(w) * f64::from(C_VZ_FOUR);
    ((a - b) - f64::from(C_VZ_BIAS)) as f32
}

/// `0x674c50` from `0x674df6` to `0x674e0d`: the tilt factor `-((dz * 0.5) / span)` rounded at the
/// `fst [ebp+8]`.
pub fn rainspawn_wind_factor(dz_cell: f32, span: f32) -> f32 {
    (-((f64::from(dz_cell) * f64::from(C_HALF)) / f64::from(span))) as f32
}

/// The same factor as the `fst` leaves it on st0, wide, for the first product of the next step.
pub fn rainspawn_wind_factor_live(dz_cell: f32, span: f32) -> f64 {
    -((f64::from(dz_cell) * f64::from(C_HALF)) / f64::from(span))
}

/// The three wind products from `0x674e0d` to `0x674e26`.
pub struct RainWindApply {
    pub out0: f32,
    pub out1: f32,
    pub out2: f32,
}

/// `0x674c50` from `0x674e0d` to `0x674e26`: the factor times the wind basis. The z product uses
/// the wide factor, the y and x products the stored one.
pub fn rainspawn_wind_apply(factor_live: f64, factor_f32: f32, basis: [f32; 3]) -> RainWindApply {
    RainWindApply {
        out2: (factor_live * f64::from(basis[2])) as f32,
        out1: (f64::from(factor_f32) * f64::from(basis[1])) as f32,
        out0: (f64::from(factor_f32) * f64::from(basis[0])) as f32,
    }
}

/// `0x674c50` from `0x674e2b` to `0x674e4e`: `pos - wind_off` per axis, rounded at the store.
pub fn rainspawn_wind_delta(pos: f32, wind_off: f32) -> f32 {
    (f64::from(pos) - f64::from(wind_off)) as f32
}

/// The sky rotation values from `0x674e66` to `0x674eb0`.
pub struct RainSkyTime {
    pub cos: f32,
    pub sin: f32,
    pub angle: f32,
}

/// `0x674c50` from `0x674e66` to `0x674eb0`: the cosine and sine of `[global+0x78] - pi/2` and the
/// angle `[global+0x7c] / 18` for the rotation builder.
pub fn rainspawn_sky_time(global78: f32, global7c: f32) -> RainSkyTime {
    let theta = f64::from(global78) - f64::from(C_SKY_BIAS);
    RainSkyTime {
        cos: theta.cos() as f32,
        sin: theta.sin() as f32,
        angle: (f64::from(global7c) * f64::from(C_SPEED_SCALE)) as f32,
    }
}

/// `0x674c50` from `0x674f6b` to `0x674fb1`: a wind anchor component times 1.75, rounded.
pub fn rainspawn_world_scale(v: f32) -> f32 {
    (f64::from(v) * f64::from(C_WORLD_SCALE)) as f32
}

/// The world position of `0x674c50` from `0x674fb6` to `0x674ff1`.
pub struct RainCamGlobal {
    pub out_x: f32,
    pub out_y: f32,
    pub out_z: f32,
}

/// `0x674c50` from `0x674fb6` to `0x674ff1`: the scaled anchor plus the camera plus the global. The
/// x and y sums are stored with `fst` and the global is added to the wide sums; the z sum is stored
/// and the global is added to the stored value.
pub fn rainspawn_cam_global(
    world: [f32; 3],
    cam: [f32; 3],
    g0: f32,
    g1: f32,
    g2: f32,
) -> RainCamGlobal {
    let sx = f64::from(world[0]) + f64::from(cam[0]);
    let sy = f64::from(world[1]) + f64::from(cam[1]);
    let sz_f32 = (f64::from(world[2]) + f64::from(cam[2])) as f32;
    RainCamGlobal {
        out_x: (sx + f64::from(g0)) as f32,
        out_y: (sy + f64::from(g1)) as f32,
        out_z: (f64::from(sz_f32) + f64::from(g2)) as f32,
    }
}

/// `0x674c50` from `0x674ff1` to `0x675013`: the stored factor times a wind basis component.
pub fn rainspawn_wind_scale2(factor: f32, basis: f32) -> f32 {
    (f64::from(factor) * f64::from(basis)) as f32
}

/// `0x674c50` from `0x675018` to `0x67503a`: `wind_off + pos` per axis.
pub fn rainspawn_pos_offset(wind_off: f32, pos: f32) -> f32 {
    (f64::from(wind_off) + f64::from(pos)) as f32
}

/// `0x674c50` from `0x6750ad` to `0x6750cf`: the factor times a wind basis component on the
/// collision leg.
pub fn rainspawn_collide_wind(factor: f32, basis: f32) -> f32 {
    (f64::from(factor) * f64::from(basis)) as f32
}

/// `0x674c50` from `0x6750d4` to `0x6750f7`: a product times the selector `[ebp-4]`.
pub fn rainspawn_collide_scale(v: f32, scale: f32) -> f32 {
    (f64::from(v) * f64::from(scale)) as f32
}

/// `0x674c50` from `0x6750fc` to `0x67511f`: a product plus the position.
pub fn rainspawn_collide_pos(v: f32, pos: f32) -> f32 {
    (f64::from(v) + f64::from(pos)) as f32
}

/// `0x674c50` from `0x675181` to `0x6751c3`: `sqrt(vel . vel)` of `0x4549f0`, rounded at the `fstp
/// [ebp+8]`.
pub fn rainspawn_velmag(vel: [f32; 3]) -> f32 {
    dot3_pc53(vel[0], vel[1], vel[2]).sqrt() as f32
}

/// `0x674c50` in the same stretch: `1.0 / |vel|` of the stored value, rounded.
pub fn rainspawn_inv_velmag(velmag_f32: f32) -> f32 {
    (f64::from(C_ONE) / f64::from(velmag_f32)) as f32
}

/// `0x674c50` in the same stretch: `hit - target` per axis, rounded at the vector store.
pub fn rainspawn_hit_delta(hit: f32, target: f32) -> f32 {
    (f64::from(hit) - f64::from(target)) as f32
}

/// The residual of `0x674c50` from `0x6751c3` to `0x675201`.
pub struct RainResidual {
    pub res0: f32,
    pub res1: f32,
    pub res2: f32,
}

/// `0x674c50` from `0x6751c3` to `0x675201`: `T = sqrt(delta . delta) * inv_velmag` stays wide
/// through the three products `T * vel`, and each residual is `delta - T * vel`, rounded.
pub fn rainspawn_residual(delta: [f32; 3], vel: [f32; 3], inv_velmag: f32) -> RainResidual {
    let t = dot3_pc53(delta[0], delta[1], delta[2]).sqrt() * f64::from(inv_velmag);
    let p0 = t * f64::from(vel[0]);
    let p1 = t * f64::from(vel[1]);
    let p2 = t * f64::from(vel[2]);
    RainResidual {
        res2: (f64::from(delta[2]) - p2) as f32,
        res1: (f64::from(delta[1]) - p1) as f32,
        res0: (f64::from(delta[0]) - p0) as f32,
    }
}

/// `0x674c50` from `0x67521c` to `0x67523c`: `hit - target` per axis, rounded at the store.
pub fn rainspawn_final_delta(hit: f32, target: f32) -> f32 {
    (f64::from(hit) - f64::from(target)) as f32
}

/// `0x674c50` from `0x67523c` to `0x675251`: `sqrt(delta . delta) * inv_velmag + bias`
/// (`[esi+0x1c]`), the square root wide into the multiply, rounded at the `fstp [esi+0x20]`.
pub fn rainspawn_lifetime(delta: [f32; 3], inv_velmag: f32, bias: f32) -> f32 {
    let dist = dot3_pc53(delta[0], delta[1], delta[2]).sqrt();
    (dist * f64::from(inv_velmag) + f64::from(bias)) as f32
}

/// The squared length of `0x4549f0`, `(a² + b²) + c²` wide.
fn dot3_pc53(a: f32, b: f32, c: f32) -> f64 {
    let (a, b, c) = (f64::from(a), f64::from(b), f64::from(c));
    (a * a + b * b) + c * c
}
