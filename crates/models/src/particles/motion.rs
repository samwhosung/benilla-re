//! One particle's step (`systems/models/particles.md#particle-motion`,
//! `systems/models/particles.md#particle-life`, `systems/models/particles.md#model-particles`):
//! `CParticleEmitter2::IntegrateParticle` (`0x7b2680`), `CParticleEmitter2::IntegrateModelParticle`
//! (`0x7b28e0`), `ParticleQuatMultiply` (`0x7b29c0`) and the age of `CParticleEmitter2::Step`
//! (`0x7b5a10`).

/// 0.0 (`0x7ffd74`).
const ZERO: f32 = 0.0;
/// 1e-8 (`0x81d86c`, `0x322bcc77`): a velocity component under this becomes 0.
const DENORM_EPS: f32 = f32::from_bits(0x322b_cc77);
/// 0.5 (`0x7ffa24`).
const HALF: f32 = 0.5;
/// 1.0 (`0x7ff9d8`).
const ONE: f32 = 1.0;
/// 1e-4 (`0x80c4f4`, `0x38d1b717`): the least spin that turns a model particle.
const SPIN_EPS: f32 = f32::from_bits(0x38d1_b717);

/// The flush after each velocity update: a nonzero component (`test ah,0x44; jnp`) whose
/// magnitude is under 1e-8 (`test ah,5; jp`) becomes 0; 0, −0 and NaN stay.
fn denorm_flush(c: f32) -> f32 {
    if c != ZERO && c.abs() < DENORM_EPS {
        ZERO
    } else {
        c
    }
}

/// `0x7b29c0` (ecx = out, edx = a, then b; quaternions `[x, y, z, w]`): `a·b`, each component one
/// register sum stored as `f32`.
pub fn particle_quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a.map(f64::from);
    let [bx, by, bz, bw] = b.map(f64::from);
    let x = (aw * bx + ay * bz + bw * ax) - by * az;
    let y = (ay * bw + bx * az + aw * by) - ax * bz;
    let z = (by * ax + bw * az + aw * bz) - ay * bx;
    let w = ((aw * bw - ax * bx) - by * ay) - az * bz;
    [x as f32, y as f32, z as f32, w as f32]
}

/// A particle as the step reads it: position `+0x0`, flag byte `+0xd`, velocity `+0x10`, age
/// `+0x1c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub flag: u8,
    pub age: f32,
}

/// The emitter fields the step reads.
#[derive(Clone, Copy, Debug)]
pub struct EmitterIntegrate {
    /// `+0x180`.
    pub gravity: f32,
    /// `+0x1ac`.
    pub flag_word: u32,
    /// `+0x1e0`.
    pub drag: f32,
    /// `+0x1e4` to `+0x1ec`: the wind.
    pub accel: [f32; 3],
    /// `+0x1f0`: the wind time.
    pub lifespan_gate: f32,
    /// `+0x278` to `+0x280`: the displacement of this substep.
    pub drift: [f32; 3],
}

/// `0x7b2680` (ecx = the emitter; the particle and `dt`; `ret 8`). In order: under the wind time
/// (`test ah,5; jp`, so a NaN age skips) `v += dt·wind`, z's product stored as `f32` first, then
/// the flush; with flag 0x40000 the displacement, or on the first step clearing flag bit 0; `p +=
/// dt·v` with `z = (z − dt·g·dt·0.5) + dt·vz` and `vz −= dt·g`; for a drag whose bits are nonzero,
/// `f = dt·drag` capped at 1 (a NaN `f` stays) and `v −= f·v`, x and y with the product stored as
/// `f32`; the flush again. With flag 0x800 it returns 0 when `(dt·vx·x + dt·vy·y) + dt·vz·z > 0`
/// (`test ah,0x41; jne`, the velocity before the drag, the new position), else 1.
pub fn particle_integrate(emitter: &EmitterIntegrate, p: &mut Particle, dt: f32) -> u32 {
    let dt = f64::from(dt);
    if p.age < emitter.lifespan_gate {
        let daz = (dt * f64::from(emitter.accel[2])) as f32;
        p.vel[0] = (dt * f64::from(emitter.accel[0]) + f64::from(p.vel[0])) as f32;
        p.vel[1] = (dt * f64::from(emitter.accel[1]) + f64::from(p.vel[1])) as f32;
        p.vel[2] = (f64::from(daz) + f64::from(p.vel[2])) as f32;
        p.vel = p.vel.map(denorm_flush);
    }
    if emitter.flag_word & 0x0004_0000 != 0 {
        if p.flag & 1 != 0 {
            p.flag &= 0xfe;
        } else {
            for k in 0..3 {
                p.pos[k] = (f64::from(p.pos[k]) + f64::from(emitter.drift[k])) as f32;
            }
        }
    }
    let [dvx, dvy, dvz] = p.vel.map(|v| dt * f64::from(v));
    let g = f64::from(emitter.gravity);
    p.pos[0] = (dvx + f64::from(p.pos[0])) as f32;
    p.pos[1] = (dvy + f64::from(p.pos[1])) as f32;
    p.pos[2] = ((f64::from(p.pos[2]) - dt * g * dt * f64::from(HALF)) + dvz) as f32;
    p.vel[2] = (f64::from(p.vel[2]) - dt * g) as f32;
    if emitter.drag.to_bits() != 0 {
        let mut f = dt * f64::from(emitter.drag);
        if f64::from(ONE) < f {
            f = f64::from(ONE);
        }
        let fvx = (f * f64::from(p.vel[0])) as f32;
        let fvy = (f * f64::from(p.vel[1])) as f32;
        p.vel[0] = (f64::from(p.vel[0]) - f64::from(fvx)) as f32;
        p.vel[1] = (f64::from(p.vel[1]) - f64::from(fvy)) as f32;
        p.vel[2] = (f64::from(p.vel[2]) - f * f64::from(p.vel[2])) as f32;
    }
    p.vel = p.vel.map(denorm_flush);
    if emitter.flag_word & 0x0000_0800 != 0 {
        let dot =
            (dvx * f64::from(p.pos[0]) + dvy * f64::from(p.pos[1])) + dvz * f64::from(p.pos[2]);
        if dot > 0.0 {
            return 0;
        }
    }
    1
}

/// A model particle's turn: the orientation `+0x20` and the spin `+0x30`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spin {
    pub orient: [f32; 4],
    pub spin: [f32; 3],
}

/// `0x7b28e0` (ecx = the emitter; the particle and `dt`; `ret 8`): `m = √((x² + y²) + z²)` of the
/// spin, compared in the register after `fst`; above 1e-4 the orientation becomes `orient·q` with
/// `h = f32(m·dt·0.5)`, `q = (s·ω, cos h)` and `s = (1/m)·sin h` from the stored `m`, `fsincos`
/// stored as `f32`. Then the step of `0x7b2680`.
pub fn particle_spin_integrate(
    emitter: &EmitterIntegrate,
    spin: &mut Spin,
    p: &mut Particle,
    dt: f32,
) -> u32 {
    let [sx, sy, sz] = spin.spin.map(f64::from);
    let mag_live = ((sx * sx + sy * sy) + sz * sz).sqrt();
    let mag = mag_live as f32;
    if mag_live > f64::from(SPIN_EPS) {
        let theta = (f64::from(mag) * f64::from(dt) * f64::from(HALF)) as f32;
        let (sin, cos) = f64::from(theta).sin_cos();
        let (sin, cos) = (sin as f32, cos as f32);
        let s = (f64::from(ONE) / f64::from(mag)) * f64::from(sin);
        let q = [(s * sx) as f32, (s * sy) as f32, (s * sz) as f32, cos];
        spin.orient = particle_quat_mul(spin.orient, q);
    }
    particle_integrate(emitter, p, dt)
}

/// The age of `0x7b5a10` (`fadd@0x7b5a90`): `dt + age`, stored as `f32` with `fst`; the death
/// test compares the register with the lifespan.
pub fn particle_time_advance(t: f32, field_1c: f32) -> f32 {
    (f64::from(t) + f64::from(field_1c)) as f32
}
