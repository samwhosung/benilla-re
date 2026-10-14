//! The drift cloud (`systems/terrain/liquid.md#drift-cloud`): the particle count `0x68e650`, the
//! gust
//! direction `0x68e1c0`, the gust velocity `0x68e4f0`, the scatter `0x68e720` and the advance
//! `0x68e930`.

use benilla_re_random::Stream;

use crate::x87::ftol;

/// `[0x81038c]`, 4000.
const C_4000: f32 = f32::from_bits(0x457a_0000);

/// `0x68e650(x)` (`ecx` = the cloud, `ret 4`): stores `__ftol(x·4000)` at `+0xfa00`, the product
/// taken at 53 bits and truncated toward zero; NaN and a value beyond `i64` store 0.
pub fn drift_cloud_count(x: f32) -> i32 {
    ftol(f64::from(x) * f64::from(C_4000))
}

/// `[0x801628]` = 2.0: the pivot of the angle remap.
const C_2_0: f32 = 2.0;
/// `[0x8030b0]` = pi: the angle scale.
const C_PI: f32 = std::f32::consts::PI;
/// `[0x8029b0]` = 0.25: the tilt scale on the gust direction's z.
const C_0_25: f32 = 0.25;
/// `[0x807a4c]` = `0x3c4ccccd` (0.0125): the gust frequency scale.
const C_FREQ: f32 = f32::from_bits(0x3c4c_cccd);
/// `[0x807a3c]` = `0x3ba3d70a` (0.005): the gust amplitude scale.
const C_AMP: f32 = f32::from_bits(0x3ba3_d70a);
/// `[0x7ff9d8]` = 1.0.
const C_1_0: f32 = 1.0;
/// `[0x7ffa24]` = 0.5: the half of the box edge and the gust phase limit.
const C_0_5: f32 = 0.5;
/// `[0x80295c]` = `0x40c90fdb` (2 pi): the gust phase to the sine argument.
const C_TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
/// `[0x80308c]` = 1.5: the upper bound of a particle's scale, times the rate.
const C_1_5: f32 = 1.5;
/// `[0x86a098]` = `0xbca3d70a` (-0.02): the vertical wind of mode 2.
const C_MODE2: f32 = f32::from_bits(0xbca3_d70a);

/// The header of the drift cloud at `0xc63180` that the four kernels read and write; the particles
/// are the 4000 records of four floats (x, y, z, scale) from offset 0.
#[derive(Clone, Copy, Debug, Default)]
pub struct DriftCloud {
    /// `+0xfa00`, the active particle count.
    pub count: u32,
    /// `+0xfa04`, the camera position at the previous advance.
    pub last_cam: [f32; 3],
    /// `+0xfa14`, the byte that gates the advance.
    pub enabled: bool,
    /// `+0xfa18`, the base of a particle's scale.
    pub spawn_rate: f32,
    /// `+0xfa1c`, the edge of the box the particles wrap in.
    pub box_extent: f32,
    /// `+0xfa24`, the gust mode (0 and 1 gust, 2 vertical, above 2 still).
    pub mode: u32,
    /// `+0xfa28`, the gust direction.
    pub gust_dir: [f32; 3],
    /// `+0xfa34`, the gust frequency.
    pub gust_freq: f32,
    /// `+0xfa38`, the gust phase.
    pub gust_phase: f32,
    /// `+0xfa3c`, the gust amplitude.
    pub gust_amp: f32,
}

/// `acc & 0x7fffff | 0x3f800000` as f32 widened: the `[1, 2)` fill.
fn mantissa_unit(acc: u32) -> f64 {
    f64::from(f32::from_bits((acc & 0x007f_ffff) | 0x3f80_0000))
}

/// The angle remap of the two draws in `0x68e1c0`: `test eax,eax; jge` takes `m - 2` for a draw
/// with the sign bit clear and `2 - m` for one with it set, both wide.
fn angle_remap(draw: u32) -> f64 {
    let m = mantissa_unit(draw);
    if (draw as i32) < 0 {
        f64::from(C_2_0) - m
    } else {
        m - f64::from(C_2_0)
    }
}

/// `0x68e1c0` (`ecx` = the cloud, `systems/terrain/liquid.md#drift-cloud`), the whole function:
/// the gust direction, frequency and amplitude from four draws, in this order. The first gives the
/// azimuth `a = remap * pi`, stored as f32 at `[ebp-4]`; the second the elevation `e = remap * pi`,
/// kept wide. The direction is `(cos e * sin a, sin e * sin a, cos a * 0.25)` with each product
/// stored to f32 and the z negated when it is below 0 (`fcom 0.0; test ah,5; jp` falls through to
/// `fchs` only for a value below 0, so -0 and NaN keep), then divided by `sqrt((x² + y²) + z²)` of
/// the
/// stored values, each rounded to f32. The third and fourth give `((m - 1) + 1) * 0.0125` and
/// `* 0.005`, rounded at the store. The phase is zeroed (the stores at `+0xfa38`).
pub fn weather_drift_reinit(c: &mut DriftCloud, rng: &mut Stream) {
    let a = f64::from((angle_remap(rng.uint32()) * f64::from(C_PI)) as f32);
    let e = angle_remap(rng.uint32()) * f64::from(C_PI);

    let (sa, ca) = (a.sin(), a.cos());
    let (se, ce) = (e.sin(), e.cos());
    let mut dy = (se * sa) as f32;
    let mut dx = (ce * sa) as f32;
    let mut dz = (ca * f64::from(C_0_25)) as f32;
    if dz < 0.0 {
        dz = -dz;
    }
    let sum = f64::from(dx) * f64::from(dx)
        + f64::from(dy) * f64::from(dy)
        + f64::from(dz) * f64::from(dz);
    let inv = f64::from(C_1_0) / sum.sqrt();
    dx = (inv * f64::from(dx)) as f32;
    dy = (inv * f64::from(dy)) as f32;
    dz = (inv * f64::from(dz)) as f32;
    c.gust_dir = [dx, dy, dz];
    c.gust_phase = 0.0;

    let m3 = rng.uint32();
    c.gust_freq =
        ((mantissa_unit(m3) - f64::from(C_1_0) + f64::from(C_1_0)) * f64::from(C_FREQ)) as f32;
    let m4 = rng.uint32();
    c.gust_amp =
        ((mantissa_unit(m4) - f64::from(C_1_0) + f64::from(C_1_0)) * f64::from(C_AMP)) as f32;
}

/// `0x68e4f0` (`ecx` = the cloud, `systems/terrain/liquid.md#drift-cloud`), the whole function:
/// the gust velocity for the time step `dt`. A mode above 1 (`cmp; jbe`, unsigned) gives
/// `(0, 0, -0.02 * dt)` for mode 2 and zero for higher modes. Otherwise the phase becomes
/// `dt + phase`, stored as f32 while the wide sum is multiplied by the frequency; when that
/// product is above 0.5 (`fcom; test ah,0x41; jne` skips on 0.5 and below and on NaN) the
/// direction is reseeded ([`weather_drift_reinit`]) and the product replaced by 0. The speed is
/// `sin(product * 2 pi) * amplitude`: x and y reload it rounded to f32 (`fst [ebp+0xc]`), z
/// multiplies the wide value.
pub fn weather_gust_velocity(c: &mut DriftCloud, dt: f32, rng: &mut Stream) -> [f32; 3] {
    if c.mode > 1 {
        if c.mode == 2 {
            return [0.0, 0.0, (f64::from(C_MODE2) * f64::from(dt)) as f32];
        }
        return [0.0, 0.0, 0.0];
    }
    let phase_f64 = f64::from(dt) + f64::from(c.gust_phase);
    c.gust_phase = phase_f64 as f32;
    let mut term = phase_f64 * f64::from(c.gust_freq);
    if term > f64::from(C_0_5) {
        weather_drift_reinit(c, rng);
        term = 0.0;
    }
    let speed_f64 = (term * f64::from(C_TWO_PI)).sin() * f64::from(c.gust_amp);
    let speed_f32 = speed_f64 as f32;
    [
        (f64::from(speed_f32) * f64::from(c.gust_dir[0])) as f32,
        (f64::from(speed_f32) * f64::from(c.gust_dir[1])) as f32,
        (speed_f64 * f64::from(c.gust_dir[2])) as f32,
    ]
}

/// `0x68e720` (`ecx` = the cloud, `systems/terrain/liquid.md#drift-cloud`), the whole function:
/// the records of the `count` particles. Each takes three draws for z, y and x (in that order),
/// each `(m - 1) * extent - half` with `half = f32(extent * 0.5)`, and a fourth for the scale
/// `(m - 1) * span + low`, `low = f32(rate * 0.5)`, `span = f32(rate * 1.5 - low)`; every value is
/// rounded at its store. The mode `new_mode & 3` is stored at `+0xfa24`.
pub fn weather_cloud_reseed(c: &mut DriftCloud, new_mode: u32, rng: &mut Stream) -> Vec<[f32; 4]> {
    let scale_lo = (f64::from(c.spawn_rate) * f64::from(C_0_5)) as f32;
    let span = (f64::from(c.spawn_rate) * f64::from(C_1_5) - f64::from(scale_lo)) as f32;
    let half = (f64::from(c.box_extent) * f64::from(C_0_5)) as f32;
    let mut out = Vec::with_capacity(c.count as usize);
    for _ in 0..c.count {
        let d1 = rng.uint32();
        let d2 = rng.uint32();
        let d3 = rng.uint32();
        let coord = |d: u32| -> f32 {
            ((mantissa_unit(d) - f64::from(C_1_0)) * f64::from(c.box_extent) - f64::from(half))
                as f32
        };
        let x = coord(d3);
        let y = coord(d2);
        let z = coord(d1);
        let m = f64::from(rng.unit12());
        let scale = ((m - f64::from(C_1_0)) * f64::from(span) + f64::from(scale_lo)) as f32;
        out.push([x, y, z, scale]);
    }
    c.mode = new_mode & 3;
    out
}

/// `0x68e930` (`ecx` = the cloud, `systems/terrain/liquid.md#drift-cloud`), the whole function with
/// the camera `cam` (`[0xc7cf20]`) and the step `dt` (`[0xc62510]`): nothing when `+0xfa14` is 0.
/// The camera delta is `last_cam - cam`, each rounded, and `last_cam` becomes `cam`. When
/// `extent² < (d2² + d0²) + d1²` (`fcompp; test ah,5; jp` skips on NaN and on the other order) the
/// particles are rescattered ([`weather_cloud_reseed`]) and the delta is zero. Each of the first
/// `count` particles then moves by `delta + velocity` ([`weather_gust_velocity`], each sum rounded)
/// and wraps per axis: above half takes the edge off, below minus half adds it.
pub fn weather_toroidal_advect(
    c: &mut DriftCloud,
    particles: &mut Vec<[f32; 4]>,
    cam: [f32; 3],
    dt: f32,
    rng: &mut Stream,
) {
    if !c.enabled {
        return;
    }
    let half = (f64::from(c.box_extent) * f64::from(C_0_5)) as f32;
    let mut d0 = (f64::from(c.last_cam[0]) - f64::from(cam[0])) as f32;
    let mut d1 = (f64::from(c.last_cam[1]) - f64::from(cam[1])) as f32;
    let mut d2 = (f64::from(c.last_cam[2]) - f64::from(cam[2])) as f32;
    c.last_cam = cam;
    let dist2 = f64::from(d2) * f64::from(d2)
        + f64::from(d0) * f64::from(d0)
        + f64::from(d1) * f64::from(d1);
    let ext2 = f64::from(c.box_extent) * f64::from(c.box_extent);
    if ext2 < dist2 {
        let mode = c.mode;
        *particles = weather_cloud_reseed(c, mode, rng);
        d0 = 0.0;
        d1 = 0.0;
        d2 = 0.0;
    }
    let vel = weather_gust_velocity(c, dt, rng);
    let add0 = (f64::from(d0) + f64::from(vel[0])) as f32;
    let add1 = (f64::from(d1) + f64::from(vel[1])) as f32;
    let add2 = (f64::from(d2) + f64::from(vel[2])) as f32;
    let nhalf = -half;
    let wrap = |v: f32| -> f32 {
        if v > half {
            (f64::from(v) - f64::from(c.box_extent)) as f32
        } else if v < nhalf {
            (f64::from(v) + f64::from(c.box_extent)) as f32
        } else {
            v
        }
    };
    for p in particles.iter_mut().take(c.count as usize) {
        p[0] = wrap((f64::from(p[0]) + f64::from(add0)) as f32);
        p[1] = wrap((f64::from(p[1]) + f64::from(add1)) as f32);
        p[2] = wrap((f64::from(p[2]) + f64::from(add2)) as f32);
    }
}
