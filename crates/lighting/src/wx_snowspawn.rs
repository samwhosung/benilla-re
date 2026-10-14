//! The arithmetic of `0x677750` Snow::SpawnFlake (`systems/lighting/precipitation.md#snow`): the
//! cell deltas, the position seed, the wind angle and drift, the sky rotation, the world position,
//! the velocity scalings and the lifetime. The draws (`0x4531e0`, each masked to a float in [1,
//! 2)), the rotation matrix (`0x7bdd60`, `0x7bcae0`, with `0x674ba0` around it), the path queries
//! (`0x67c760` and its table) and the vector leaves `0x4549a0` and `0x4549f0` are calls this
//! arithmetic does not include; `0x4549f0` runs as its arithmetic.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`. Where the exe stores a
//! value with `fst` and goes on with the wide one, the first use takes the wide value and the later
//! uses the stored one (the drift scalar and the camera sum). `fsincos`, `fsin` and `fcos` are
//! stood in by the host's `f64` functions.

/// `[0x7ff9d8]`, 0x3f800000.
const C_ONE: f32 = 1.0;
/// `[0x7ffa24]`, 0x3f000000.
const C_HALF: f32 = 0.5;
/// `[0x7ffd74]`, 0x00000000 (the z-component is multiplied by 0.0).
const C_ZERO: f32 = 0.0;
/// `[0x801628]`, 0x40000000.
const C_TWO: f32 = 2.0;
/// `[0x80295c]`, 0x40c90fdb.
const C_TWO_PI: f32 = 6.283_185_5;
/// `[0x8030a0]`, 0xbf800000.
const C_NEG_ONE: f32 = -1.0;
/// `[0x808acc]`, 0x3fc90fdb.
const C_HALF_PI: f32 = 1.570_796_4;
/// `[0x80ffb0]`, 0x3d638e39 (~ 1/18).
const C_MTX_SCALE: f32 = 0.055_555_556;
/// `[0x80ffbc]`, 0x3fc8f5c3.
const C_ANG_BIAS: f32 = 1.57;
/// `[0x80ffe4]`, 0x3fe00000.
const C_WORLD_SCALE: f32 = 1.75;
/// `[0x80ffe8]`, 0x40600000.
const C_VEL_BIAS: f32 = 3.5;
/// `[0x80ffec]`, 0x3c75c28f.
const C_T_BIAS: f32 = 0.015;
/// `[0x80fff0]`, 0x40bf851f.
const C_T_OFF: f32 = 5.985;
/// `[0x80fff4]`, 5.934_119_7 (literal would round up).
const C_ANG_SCALE: f32 = f32::from_bits(0x40bd_e44f);

/// A draw masked as the exe does, `raw & 0x7fffff | 0x3f800000`, read as a float in [1, 2).
#[inline]
pub fn snowfl_rng_unit_float(rng_raw: u32) -> f32 {
    f32::from_bits((rng_raw & 0x007f_ffff) | 0x3f80_0000)
}

/// `0x677750` from `0x67775d` to `0x67777e`: a cell span `hi - lo`, rounded at the `fstp` (three
/// axes).
pub fn snowfl_cell_delta(hi: f32, lo: f32) -> f32 {
    (f64::from(hi) - f64::from(lo)) as f32
}

/// The position components of `0x677750` from `0x6777a7` to `0x6777e9`.
pub struct SnowflPos {
    /// `[esp]`: `delta_x * fb`.
    pub out_x: f32,
    /// `[esp+4]`: `delta_y * fa`.
    pub out_y: f32,
    /// `[esp+8]`: `delta_z * 0.0`.
    pub out_z: f32,
}

/// `0x677750` from `0x6777a7` to `0x6777e9`: two draws `(rng - 1) - 0.5` kept wide multiply the
/// cell deltas, x by the second and y by the first; z is `delta_z * 0.0`. Each is rounded.
pub fn snowfl_pos_seed(
    rng1: f32,
    rng2: f32,
    delta_x: f32,
    delta_y: f32,
    delta_z: f32,
) -> SnowflPos {
    let fb = (f64::from(rng2) - f64::from(C_ONE)) - f64::from(C_HALF);
    let fa = (f64::from(rng1) - f64::from(C_ONE)) - f64::from(C_HALF);
    SnowflPos {
        out_z: (f64::from(delta_z) * f64::from(C_ZERO)) as f32,
        out_y: (f64::from(delta_y) * fa) as f32,
        out_x: (f64::from(delta_x) * fb) as f32,
    }
}

/// `0x677750` from `0x6777f8` to `0x67783d`: the wind angle `(2 pi - m * 5.9341197) * ((rng3 - 1) -
/// 0.5) - 1.57`, with `m` the weather value `[esi+0xa4]`, rounded at the `fstp [ebp-0x20]`.
pub fn snowfl_angle1(m: f32, rng3: f32) -> f32 {
    let r = f64::from(C_TWO_PI) - f64::from(m) * f64::from(C_ANG_SCALE);
    let k = (f64::from(rng3) - f64::from(C_ONE)) - f64::from(C_HALF);
    (r * k - f64::from(C_ANG_BIAS)) as f32
}

/// The angle and its cosine and sine from `0x6777f8` to `0x67784c`.
pub struct SnowflSinCos {
    /// `[ebp-0x20]`: the angle stored as f32, which feeds `fsincos`.
    pub angle: f32,
    /// `[ebp-0x44]`: the cosine.
    pub cos: f32,
    /// `[ebp-0x40]`: the sine.
    pub sin: f32,
}

/// `0x677750` from `0x6777f8` to `0x67784c`: the stored angle reloaded as f32 for `fsincos`.
pub fn snowfl_sincos(m: f32, rng3: f32) -> SnowflSinCos {
    let angle = snowfl_angle1(m, rng3);
    let a = f64::from(angle);
    SnowflSinCos {
        angle,
        cos: a.cos() as f32,
        sin: a.sin() as f32,
    }
}

/// `0x677750` from `0x677863` to `0x67788c`: the drift phase `(((rng4 - 1) - 0.5) + 5.985) * m +
/// 0.015`, rounded at its store.
pub fn snowfl_phase(m: f32, rng4: f32) -> f32 {
    ((((f64::from(rng4) - f64::from(C_ONE)) - f64::from(C_HALF)) + f64::from(C_T_OFF))
        * f64::from(m)
        + f64::from(C_T_BIAS)) as f32
}

/// The drift values from `0x67789b` to `0x6778f5`.
pub struct SnowflDrift {
    /// `[ebp-0xc]`: `t * sin`.
    pub drift_sin: f32,
    /// `[ebp-8]`: `t * cos`.
    pub drift_cos: f32,
    /// `[ebp-4]`: the vertical term.
    pub u: f32,
}

/// `0x677750` from `0x67789b` to `0x6778f5`: `t * sin` and `t * cos` (the stored phase), and the
/// vertical term `u = -((rng5 - 1) * m) - m * 3.5 - 2`, each rounded.
pub fn snowfl_drift(t: f32, sin: f32, cos: f32, m: f32, rng5: f32) -> SnowflDrift {
    let drift_sin = (f64::from(t) * f64::from(sin)) as f32;
    let drift_cos = (f64::from(t) * f64::from(cos)) as f32;
    let neg = ((f64::from(rng5) - f64::from(C_ONE)) * f64::from(m)) * f64::from(C_NEG_ONE);
    let u = (neg - f64::from(m) * f64::from(C_VEL_BIAS) - f64::from(C_TWO)) as f32;
    SnowflDrift {
        drift_sin,
        drift_cos,
        u,
    }
}

/// The three drift velocity products from `0x6778f5` to `0x677925`.
pub struct SnowflDriftVel {
    /// `[esp]`: the stored `d` times the x velocity.
    pub out_x: f32,
    /// `[esp+4]`: the stored `d` times the y velocity.
    pub out_y: f32,
    /// `[esp+8]`: the wide `d` times `u`.
    pub out_z: f32,
}

/// `0x677750` from `0x6778f5` to `0x677925`: the drift scalar `d = -((delta_z * 0.5) / u)` is
/// stored with `fst [ebp+8]`; the z product uses the wide `d` and the x and y products the stored
/// one.
pub fn snowfl_drift_vel(delta_z: f32, u: f32, vel0: f32, vel1: f32) -> SnowflDriftVel {
    let d_live = -((f64::from(delta_z) * f64::from(C_HALF)) / f64::from(u));
    let d_f32 = d_live as f32;
    SnowflDriftVel {
        out_z: (d_live * f64::from(u)) as f32,
        out_y: (f64::from(d_f32) * f64::from(vel1)) as f32,
        out_x: (f64::from(d_f32) * f64::from(vel0)) as f32,
    }
}

/// `0x677750` from `0x67792a` to `0x67794d`: `base - drift` per axis, rounded at the vector store.
pub fn snowfl_sub_drift(base: f32, drift: f32) -> f32 {
    (f64::from(base) - f64::from(drift)) as f32
}

/// The sky rotation cosine and sine from `0x67796a` to `0x677986`.
pub struct SnowflSkyRot {
    /// `[ebp-0x50]`.
    pub cos: f32,
    /// `[ebp-0x4c]`.
    pub sin: f32,
}

/// `0x677750` from `0x67796a` to `0x677986`: the cosine and sine of the sky angle less pi/2.
pub fn snowfl_sky_rot(sky_angle: f32) -> SnowflSkyRot {
    let a = f64::from(sky_angle) - f64::from(C_HALF_PI);
    SnowflSkyRot {
        cos: a.cos() as f32,
        sin: a.sin() as f32,
    }
}

/// `0x677750` from `0x677986` to `0x6779af`: `[eax+0x7c]` times 1/18, rounded at the store.
pub fn snowfl_mtx_scale(v: f32) -> f32 {
    (f64::from(v) * f64::from(C_MTX_SCALE)) as f32
}

/// `0x677750` from `0x677a58` to `0x677a86`: a wind anchor component times 1.75, rounded.
pub fn snowfl_world_scale(v: f32) -> f32 {
    (f64::from(v) * f64::from(C_WORLD_SCALE)) as f32
}

/// The world position of `0x677750` from `0x677a8b` to `0x677ac5`.
pub struct SnowflCamGlobal {
    /// `[esi]`.
    pub out_x: f32,
    /// `[esi+4]`.
    pub out_y: f32,
    /// `[esi+8]`, also kept at `[ebp+0x14]`.
    pub out_z: f32,
}

/// `0x677750` from `0x677a8b` to `0x677ac5`: the scaled anchor plus the camera plus the global. The
/// x and y sums are stored with `fst` and the global is added to the wide sums; the z sum is stored
/// as f32 and the global is added to the stored value.
pub fn snowfl_cam_global(
    world: [f32; 3],
    cam: [f32; 3],
    g0: f32,
    g1: f32,
    g2: f32,
) -> SnowflCamGlobal {
    let sx = f64::from(world[0]) + f64::from(cam[0]);
    let sy = f64::from(world[1]) + f64::from(cam[1]);
    let sz_f32 = (f64::from(world[2]) + f64::from(cam[2])) as f32;
    SnowflCamGlobal {
        out_x: (sx + f64::from(g0)) as f32,
        out_y: (sy + f64::from(g1)) as f32,
        out_z: (f64::from(sz_f32) + f64::from(g2)) as f32,
    }
}

/// `0x677750` from `0x677ac5` to `0x677ae4`: the stored scalar `[ebp+8]` times a velocity
/// component.
pub fn snowfl_vel_z_scale(s: f32, v: f32) -> f32 {
    (f64::from(s) * f64::from(v)) as f32
}

/// `0x677750` from `0x677ae9` to `0x677b0b`: `a + b` per axis, rounded at the vector store.
pub fn snowfl_add_offset(a: f32, b: f32) -> f32 {
    (f64::from(a) + f64::from(b)) as f32
}

/// `0x677750` from `0x677b6d` to `0x677b8f`: the stored scalar `[ebp+8]` times a velocity
/// component.
pub fn snowfl_vel_drift_scale(s: f32, v: f32) -> f32 {
    (f64::from(s) * f64::from(v)) as f32
}

/// `0x677750` from `0x677b94` to `0x677bb7`: a product times the collision flag `[ebp+0x14]`, 2.0
/// or 1.0.
pub fn snowfl_vel_flag_scale(v: f32, flag: f32) -> f32 {
    (f64::from(v) * f64::from(flag)) as f32
}

/// `0x677750` from `0x677bbc` to `0x677bdf`: `v + base` per axis, rounded at the vector store.
pub fn snowfl_add_base(v: f32, base: f32) -> f32 {
    (f64::from(v) + f64::from(base)) as f32
}

/// The squared length of `0x4549f0`, `(a² + b²) + c²` wide.
fn snowfl_dot3_pc53(a: f32, b: f32, c: f32) -> f64 {
    let (a, b, c) = (f64::from(a), f64::from(b), f64::from(c));
    (a * a + b * b) + c * c
}

/// `0x677750` from `0x677c3a` to `0x677d05`: `speed = sqrt(|vel|²)` and `invspeed = 1 / speed` are
/// stored as f32. The path fraction `r = sqrt(|pos - hit|²) * invspeed` stays wide through the
/// three products `r * vel`; the new point `hit - r * vel` and its distance to `hit` are stored as
/// f32, and the lifetime is `sqrt(|e|²) * invspeed + param`, rounded at the `fstp [esi+0x1c]`.
pub fn snowfl_lifetime(pos: [f32; 3], hit: [f32; 3], vel: [f32; 3], param: f32) -> f32 {
    let speed = (snowfl_dot3_pc53(vel[0], vel[1], vel[2]).sqrt()) as f32;
    let d0 = (f64::from(pos[0]) - f64::from(hit[0])) as f32;
    let d1 = (f64::from(pos[1]) - f64::from(hit[1])) as f32;
    let d2 = (f64::from(pos[2]) - f64::from(hit[2])) as f32;
    let invspeed = (f64::from(C_ONE) / f64::from(speed)) as f32;
    let r = snowfl_dot3_pc53(d0, d1, d2).sqrt() * f64::from(invspeed);
    let np0 = (f64::from(hit[0]) - r * f64::from(vel[0])) as f32;
    let np1 = (f64::from(hit[1]) - r * f64::from(vel[1])) as f32;
    let np2 = (f64::from(hit[2]) - r * f64::from(vel[2])) as f32;
    let e0 = (f64::from(np0) - f64::from(hit[0])) as f32;
    let e1 = (f64::from(np1) - f64::from(hit[1])) as f32;
    let e2 = (f64::from(np2) - f64::from(hit[2])) as f32;
    (snowfl_dot3_pc53(e0, e1, e2).sqrt() * f64::from(invspeed) + f64::from(param)) as f32
}
