//! How an emitter spawns a particle (`systems/models/emission.md`): the shared speed and age of
//! `CParticleEmitter2::RollSpeed` (`0x7b1cc0`), the shapes `PlaneEmitterSpawnParticle`
//! (`0x7b8890`), `SphereEmitterSpawnParticle` (`0x7b8d70`) and `SplineEmitterSpawnParticle`
//! (`0x7b9500`), their setters (`0x7b9180`, `0x7b91a0`, `0x7b99c0`, `0x7b9a60`), the ground snap of
//! `CParticleEmitter2::SnapSpawnToGround` (`0x7b2140`), the follow mapping of
//! `CParticleEmitter2::SetFollowMapping` (`0x7b5d30`) and the z source of
//! `CParticleEmitter2::SetZSource` (`0x7b5cd0`).
//!
//! The draws come from the emitter's own `CRandom` at `+0x2c` (`systems/random.md`), so the shapes
//! are run with that state seeded and stepped alongside. The cone and sphere directions go through
//! `fsin` and `fcos`, which the host does not reproduce bit for bit; those are not modelled.

use benilla_re_random::{crandom_uint32, srand_seed_state};

/// 2.3841858e-7 (`0x8029d4`).
const NORM_EPS: f32 = f32::from_bits(0x3480_0000);

/// The table `CRandom::uint32` reads, `0x802700`, and how many bytes of it a draw can reach.
pub const PRNG_TABLE_VA: u32 = benilla_re_random::TABLE_VA;
pub const PRNG_TABLE_LEN: usize = 0x100 + 3;

/// An emitter's `CRandom`: the accumulator at `+0x2c` and the lanes at `+0x30`.
#[derive(Clone, Copy)]
pub struct CRandom {
    pub state: [u32; 2],
    table: [u8; PRNG_TABLE_LEN],
}

impl CRandom {
    /// `CRndSeed::SetSeed` (`0x44ea80`).
    pub fn from_seed(seed: u32, table: [u8; PRNG_TABLE_LEN]) -> CRandom {
        let (a, b) = srand_seed_state(seed);
        CRandom {
            state: [a, b],
            table,
        }
    }

    /// A state already in memory.
    pub fn from_state(state: [u32; 2], table: [u8; PRNG_TABLE_LEN]) -> CRandom {
        CRandom { state, table }
    }

    /// `CRandom::uint32` (`0x4531e0`): the new accumulator.
    pub fn next_u32(&mut self) -> u32 {
        let (acc, lanes, eax) = crandom_uint32(self.state[0], self.state[1], &self.table);
        self.state = [acc, lanes];
        eax
    }

    /// The bits `(w & 0x7fffff) | 0x3f800000`, a float in `[1, 2)`.
    pub fn unit_1_2(raw: u32) -> f32 {
        f32::from_bits((raw & 0x007f_ffff) | 0x3f80_0000)
    }

    /// `S` of `systems/models/emission.md#speed-and-birth`: `2 − x` for a negative word, else `x −
    /// 2`, exact as `f32`.
    pub fn signed_unit(raw: u32) -> f32 {
        let u = CRandom::unit_1_2(raw);
        if (raw as i32) < 0 {
            2.0 - u
        } else {
            u - 2.0
        }
    }
}

/// `0x7b1cc0` (`systems/models/emission.md#speed-and-birth`): one draw from the emitter's state,
/// then `(S·variation + 1)·speed` left in the register (`+0x184` the variation, `+0x17c` the
/// speed).
pub fn emitter_speed(rng: &mut CRandom, base: f32, variation: f32) -> f64 {
    let s = CRandom::signed_unit(rng.next_u32());
    (f64::from(s) * f64::from(variation) + 1.0) * f64::from(base)
}

/// The age every shape gives a new particle: `(x − 1)·dt` from the first draw, stored as `f32`.
pub fn spawn_age_offset(draw0: u32, scale: f32) -> f32 {
    ((f64::from(CRandom::unit_1_2(draw0)) - 1.0) * f64::from(scale)) as f32
}

/// The plane emitter's fields.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlaneEmitter {
    /// `+0x290`, `+0x294`: length and width.
    pub half_x: f32,
    pub half_y: f32,
    /// `+0x298`, `+0x29c`: the cone's spreads.
    pub spread_a: f32,
    pub spread_b: f32,
    /// `+0x188`: the z source.
    pub z_pivot: f32,
    /// `+0x1ac`.
    pub flags: u32,
}

/// A plane particle's start and, with a z source, its direction.
pub struct PlaneSpawn {
    pub vel: [f32; 3],
    pub radial_dir: [f32; 3],
}

/// `0x7b8890` (`systems/models/emission.md#plane-spawn`): the start `(S2·length·0.5, S1·width·0.5,
/// 0)` from the second and first draws after the age, each stored as `f32`; with a z source the
/// direction `t = p − (0, 0, z)` times `speed/√((x² + y²) + z²)`, each stored as `f32`.
pub fn emit_shape_plane(e: &PlaneEmitter, draw1: u32, draw2: u32, speed: f32) -> PlaneSpawn {
    let vx = ((f64::from(CRandom::signed_unit(draw2)) * f64::from(e.half_x)) * 0.5) as f32;
    let vy = ((f64::from(CRandom::signed_unit(draw1)) * f64::from(e.half_y)) * 0.5) as f32;
    let vel = [vx, vy, 0.0f32];
    let t = [f64::from(vx), f64::from(vy), f64::from(0.0f32 - e.z_pivot)];
    let inv = f64::from(speed) / ((t[0] * t[0] + t[1] * t[1]) + t[2] * t[2]).sqrt();
    PlaneSpawn {
        vel,
        radial_dir: t.map(|c| (c * inv) as f32),
    }
}

/// The burst of `systems/models/emission.md#spawn-space`: `m = S·variation + 1` in the register,
/// then the emitter's velocity times `m` added to `dir`: x unrounded, y and z stored as `f32` first
/// (`0x7b8b84` to `0x7b8bac`).
pub fn burst_velocity(dir: [f32; 3], draw: u32, variation: f32, burst_dir: [f32; 3]) -> [f32; 3] {
    let mag = f64::from(CRandom::signed_unit(draw)) * f64::from(variation) + 1.0;
    [
        (mag * f64::from(burst_dir[0]) + f64::from(dir[0])) as f32,
        (f64::from((mag * f64::from(burst_dir[1])) as f32) + f64::from(dir[1])) as f32,
        (f64::from((mag * f64::from(burst_dir[2])) as f32) + f64::from(dir[2])) as f32,
    ]
}

/// The sphere emitter's fields.
#[derive(Clone, Copy, Debug, Default)]
pub struct SphereEmitter {
    /// `+0x290`: the minimum radius.
    pub radius_mid: f32,
    /// `+0x298`: maximum less minimum.
    pub radius_span: f32,
    /// `+0x29c`, `+0x2a0`: the vertical and horizontal spreads.
    pub angle_a: f32,
    pub angle_b: f32,
    /// `+0x188`.
    pub z_pivot: f32,
    /// `+0x1ac`.
    pub flags: u32,
}

/// What [`emit_shape_sphere`] draws before the direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphereScalars {
    pub radius: f32,
    pub theta: f32,
    pub phi: f32,
}

/// `0x7b8d70` (`systems/models/emission.md#sphere-spawn`): `R = (x − 1)·span + min`, `A =
/// S·vertical` and `B = S·horizontal` from the three draws after the age, each stored as `f32`.
pub fn emit_shape_sphere(e: &SphereEmitter, draw1: u32, draw2: u32, draw3: u32) -> SphereScalars {
    SphereScalars {
        radius: ((f64::from(CRandom::unit_1_2(draw1)) - 1.0) * f64::from(e.radius_span)
            + f64::from(e.radius_mid)) as f32,
        theta: CRandom::signed_unit(draw2) * e.angle_a,
        phi: CRandom::signed_unit(draw3) * e.angle_b,
    }
}

/// The spline emitter's fields.
#[derive(Clone, Copy, Debug, Default)]
pub struct SplineEmitter {
    /// `+0x2a4`: set when the end changed.
    pub dirty: u32,
    /// `+0x298`: the end.
    pub forced: f32,
    /// `+0x294`: the start.
    pub range_lo: f32,
}

/// `0x7b9500` (`systems/models/emission.md#spline-spawn`): with `+0x2a4` set the parameter is the
/// end and the flag is cleared; otherwise `(x − 1)·(end − start) + start`, the span in the
/// register. Returns `(t, cleared)`.
pub fn emit_shape_spline(e: &SplineEmitter, draw1: u32) -> (f32, bool) {
    if e.dirty != 0 {
        return (e.forced, true);
    }
    let span = f64::from(e.forced) - f64::from(e.range_lo);
    let t = (f64::from(CRandom::unit_1_2(draw1)) - 1.0) * span + f64::from(e.range_lo);
    (t as f32, false)
}

/// What [`sphere_set_mid`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphereMid {
    /// `+0x290`.
    pub mid: f32,
    /// `+0x298`: `+0x294 − mid`.
    pub half_span: f32,
}

/// `0x7b9180` (`systems/models/emission.md#sphere-spawn`): the minimum radius and `max − min`,
/// stored as `f32`.
pub fn sphere_set_mid(max: f32, mid: f32) -> SphereMid {
    SphereMid {
        mid,
        half_span: max - mid,
    }
}

/// What [`sphere_set_max`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphereMax {
    /// `+0x294`.
    pub max: f32,
    /// `+0x298`: `max − +0x290`.
    pub half_span: f32,
}

/// `0x7b91a0`: the maximum radius and `max − min`, stored as `f32`.
pub fn sphere_set_max(mid: f32, max: f32) -> SphereMax {
    SphereMax {
        max,
        half_span: max - mid,
    }
}

/// What [`spline_set_progress`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineSet {
    /// Whether `+0x2a4`, `+0x298` and `+0xa8` were written.
    pub dirty: bool,
    /// `+0x298`.
    pub recompute: f32,
    /// `+0xa8`: the end times `+0x290`.
    pub derived: f32,
}

/// `0x7b99c0` (`systems/models/emission.md#spline-spawn`): the end clamped below 0 to 0 (ordered,
/// `test ah,5; jp`) and otherwise at or above 1, NaN included, to 1 (`jnp`); then, unless it moved
/// less than 2.3841858e-7 from `+0x298` (ordered, `jnp`), `+0x2a4 = 1`, `+0x298` the end and
/// `+0xa8` the end times `+0x290`.
pub fn spline_set_progress(arg: f32, old: f32, mul290: f32) -> SplineSet {
    let clamped = if arg < 0.0 {
        0.0
    } else if !(arg < 1.0) {
        1.0
    } else {
        arg
    };
    let diff = (f64::from(clamped) - f64::from(old)).abs();
    if diff < f64::from(NORM_EPS) {
        return SplineSet {
            dirty: false,
            recompute: 0.0,
            derived: 0.0,
        };
    }
    SplineSet {
        dirty: true,
        recompute: clamped,
        derived: clamped * mul290,
    }
}

/// `0x7b9a60` (`systems/models/emission.md#spline-spawn`): the start's scale stored and `+0xa8 =
/// scale·+0x298`.
pub fn spline_set_scale(scale: f32, field_298: f32) -> f32 {
    scale * field_298
}

/// `0x7b2140` from `0x7b219c` to `0x7b21a5` (`systems/models/emission.md#spawn-space`): the
/// particle's height, the ground's height plus its current size, stored as `f32`.
pub fn color_scale_apply_sum(local_c: f32, local_8: f32) -> f32 {
    local_c + local_8
}

/// What [`remap_fit`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemapFit {
    /// `+0x1f8`.
    pub slope: f32,
    /// `+0x1f4`.
    pub intercept: f32,
}

/// `0x7b5d30(s0, f0, s1, f1)` (`systems/models/emission.md#record-to-emitter`): with `|s1 − s0|`
/// below `eps` (2.3841858e-7 at `0x8029d4`, ordered, `jnp`) both are 0; otherwise the slope `(f1 −
/// f0)/(s1 − s0)` stored by `fst` and kept, and the intercept `f0 − slope·s0` from the kept value.
pub fn remap_fit(x0: f32, y0: f32, x1: f32, y1: f32, eps: f32) -> RemapFit {
    let dx = f64::from(x1) - f64::from(x0);
    if dx.abs() < f64::from(eps) {
        return RemapFit {
            slope: 0.0,
            intercept: 0.0,
        };
    }
    let slope = (f64::from(y1) - f64::from(y0)) / dx;
    RemapFit {
        slope: slope as f32,
        intercept: (f64::from(y0) - slope * f64::from(x0)) as f32,
    }
}

/// 0.001 (`0x81d830`, `0x3a83126f`): the least `|z|` the z source keeps.
pub const FLUSH_EPS: f32 = f32::from_bits(0x3a83_126f);

/// `CParticleEmitter2::SetZSource` (`0x7b5cd0`, `systems/models/emission.md#per-frame-parameters`):
/// `+0x188 = z`, then 0 when `|z| < 0.001` (`test ah,5; jp`, so 0.001 and NaN stay).
pub fn set_scalar_flush_denormal(x: f32) -> f32 {
    if f64::from(x).abs() < f64::from(FLUSH_EPS) {
        0.0
    } else {
        x
    }
}
