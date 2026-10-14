//! Arithmetic of the weather driver `0x67be40` UpdateWeather
//! (`systems/lighting/weather.md#weather-driver`) and its motion sampler `0x67c150`
//! SampleWeatherMotion (`weather.md#player-motion`), and the box test `0x6dc5a0`.
//!
//! The driver and the sampler are mostly calls, so each transcription is one stretch of them: the
//! wind speed and heading, the transport's squared and planar speed, the drift test, the position
//! steps and the velocity average. Values are f64 on the x87 stack and round to f32 at each store;
//! a value that goes straight into a compare stays wide.

const C_ONE: f32 = f32::from_bits(0x3f80_0000);
const C_MS: f32 = f32::from_bits(0x3a83_126f);
const C_EPS_TINY: f32 = f32::from_bits(0x3480_0000);
const C_BOX_EPS: f32 = f32::from_bits(0x3727_c5ac);
const C_TWO: f32 = f32::from_bits(0x4000_0000);

/// `0x6dc5a0` (`ecx` the box, `edx` the point `Q`, `[esp+4]` the point `P`; the system is not
/// assigned), the whole function: whether the segment from `Q` to `P` touches the box `[bmin,
/// bmax]`. With `diff = P - Q` stored as f32, each axis tests `Q` against the slab: `fcomp; test
/// ah,5; jp` below `bmin` returns 0 when `P` is also below it (`jnp`), else takes `bmin` as the
/// entry plane; the same above `bmax` with `test ah,0x41`. A plane gives `t = (plane - Q) / diff`
/// stored as f32 unless the bits of `diff` are 0 (so -0 divides); `t` starts at -1 and a NaN
/// coordinate counts as inside the slab. All three axes inside returns 1. Otherwise the axis with
/// the largest `t` (`fcomp; test ah,0x41; jne`, the first on ties) is the entry axis; a sign bit
/// set in its `t` returns 0. For each other axis the hit point `h = diff * t + Q` stays wide and
/// must not be below `bmin - 1e-5` or above `bmax + 1e-5`, both wide (`fcompp; test ah,5; jnp` and
/// `test ah,0x41; je`); NaN passes.
#[must_use]
pub fn ray_box_clip(p: [f32; 3], q: [f32; 3], bmin: [f32; 3], bmax: [f32; 3]) -> u32 {
    let f = f64::from;
    let diff = [
        (f(p[0]) - f(q[0])) as f32,
        (f(p[1]) - f(q[1])) as f32,
        (f(p[2]) - f(q[2])) as f32,
    ];
    let mut t = [-1.0f32, -1.0, -1.0];
    let mut inside = true;
    for i in 0..3 {
        if q[i] < bmin[i] {
            if p[i] < bmin[i] {
                return 0;
            }
            inside = false;
            if diff[i].to_bits() != 0 {
                t[i] = ((f(bmin[i]) - f(q[i])) / f(diff[i])) as f32;
            }
        } else if q[i] > bmax[i] {
            if p[i] > bmax[i] {
                return 0;
            }
            inside = false;
            if diff[i].to_bits() != 0 {
                t[i] = ((f(bmax[i]) - f(q[i])) / f(diff[i])) as f32;
            }
        }
    }
    if inside {
        return 1;
    }
    let mut k = 0usize;
    if t[1] > t[0] {
        k = 1;
    }
    if t[2] > t[k] {
        k = 2;
    }
    if (t[k].to_bits() & 0x8000_0000) != 0 {
        return 0;
    }
    for j in 0..3 {
        if j == k {
            continue;
        }
        let h = f(diff[j]) * f(t[k]) + f(q[j]);
        if h < f(bmin[j]) - f(C_BOX_EPS) {
            return 0;
        }
        if h > f(bmax[j]) + f(C_BOX_EPS) {
            return 0;
        }
    }
    1
}

/// `0x67be40` from `0x67be8a` to `0x67bea4`: the squared horizontal wind `y² + x²` of the player
/// velocity (`[edx]`, `[edx+4]`), stored as f32 by the `fst dword [ebp-8]` while st0 stays wide.
#[must_use]
pub fn wx_wind_mag2(wind_x: f32, wind_y: f32) -> f32 {
    let f = f64::from;
    (f(wind_y) * f(wind_y) + f(wind_x) * f(wind_x)) as f32
}

/// `0x67be40` at `0x67bea4`: the `fsqrt` of that wide `y² + x²` (not of the f32 copy) goes straight
/// into `fcomp 1.0; test ah,5; jnp`, which takes the facing slot (`0x67befb`) when it is below 1.
/// At 1 or more, and for NaN, the heading is the wind's angle ([`wx_wind_angle`]) unless the unit
/// flag is set.
#[must_use]
pub fn wx_wind_mag_live_sqrt(wind_x: f32, wind_y: f32) -> f64 {
    let f = f64::from;
    (f(wind_y) * f(wind_y) + f(wind_x) * f(wind_x)).sqrt()
}

/// `0x67be40` from `0x67bec7` to `0x67bf09`: the heading. `inv = 1 / sqrt(mag2_f32)` of the f32
/// copy, then `fpatan` of `(y * inv, x * inv)`, stored at `[esi+0x78]` as f32; `fcom 0.0; test
/// ah,5; jp` adds the wrap `[0xc7b554]` (2π) only for an angle below 0 (NaN is kept).
#[must_use]
pub fn wx_wind_angle(mag2_f32: f32, wind_x: f32, wind_y: f32, wrap: f32) -> f32 {
    let f = f64::from;
    let inv = f(C_ONE) / f(mag2_f32).sqrt();
    let x_inv = f(wind_x) * inv;
    let y_inv = f(wind_y) * inv;
    let angle = y_inv.atan2(x_inv);
    if angle < 0.0 {
        (angle + f(wrap)) as f32
    } else {
        angle as f32
    }
}

/// The squared length `(a² + b²) + c²` of a vector, wide, in the order of the `fld c; fld b; fld a`
/// sequence at `0x67bf44` (the transport's velocity, `0x67be40`) and `0x67c65c` (the sampler's
/// wind): `fcomp 2.0; test ah,0x41` follows, and `jne` at `0x67bf6b` takes the other branch for 2.0
/// or less and NaN, so the transport's planar speed ([`wx_manager_planar_len`]) is used only above
/// 2.0.
#[must_use]
pub fn wx_vec3_sqlen(comp: [f32; 3]) -> f64 {
    let f = f64::from;
    let a = f(comp[0]);
    let b = f(comp[1]);
    let c = f(comp[2]);
    a * a + b * b + c * c
}

/// `0x67be40` from `0x67bf6d` to `0x67bf7d`: the transport's planar speed `sqrt(q² + p²)` of
/// `[esi+0x5c]` and `[esi+0x60]`, stored as f32 at `+0x7c`.
#[must_use]
pub fn wx_manager_planar_len(p: f32, q: f32) -> f32 {
    let f = f64::from;
    (f(q) * f(q) + f(p) * f(p)).sqrt() as f32
}

/// `0x67be40` from `0x67c0fb` and `0x67c10f`: `|a - b|`, wide, goes into `fcomp [0x8029d4]; test
/// ah,5; jp`, which takes the branch for a difference of the threshold or more and for NaN.
#[must_use]
pub fn wx_abs_diff(a: f32, b: f32) -> f64 {
    let f = f64::from;
    (f(a) - f(b)).abs()
}

/// `[0x8029d4]` = `0x34800000` (2⁻²²): the drift threshold.
pub const WX_DRIFT_EPS: f32 = C_EPS_TINY;
/// `[0x801628]` = 2.0: the squared-speed threshold.
pub const WX_DOT_THRESHOLD: f32 = C_TWO;

/// `0x67c150` at `0x67c37b` and `0x67c458`: the step of a position, `pos - prev` per axis, each
/// rounded at its `fstp` (the camera against `[0xc7b4fc]`, the listener against `[0xc7b544]`).
#[must_use]
pub fn wx_pos_delta(pos: [f32; 3], prev: [f32; 3]) -> [f32; 3] {
    let f = f64::from;
    [
        (f(pos[0]) - f(prev[0])) as f32,
        (f(pos[1]) - f(prev[1])) as f32,
        (f(pos[2]) - f(prev[2])) as f32,
    ]
}

/// `0x67c150` from `0x67c53f` and the copy at `0x67c5dd`: the velocity of a list from its summed
/// steps. `recip = 1 / ((count_total + 1) * 0.001)` with the count as an unsigned qword is stored
/// as f32; each component is `recip * sum` rounded at its store, the third from the sum stored as
/// f32 (`fstp [ebp-0x18]`) and the other two from the wide sums.
#[must_use]
pub fn wx_velocity_average(sums: [f64; 3], sum2_f32: f32, count_total: u32) -> [f32; 3] {
    let f = f64::from;
    let denom = (u64::from(count_total) + 1) as f64 * f(C_MS);
    let recip = (f(C_ONE) / denom) as f32;
    [
        (f(recip) * sums[0]) as f32,
        (f(recip) * sums[1]) as f32,
        (f(recip) * f(sum2_f32)) as f32,
    ]
}

/// `0x67c150` at `0x67c4ff` and `0x67c587`: the sums of a list's steps, each component added wide
/// in list order from 0; the third is also rounded to f32 for [`wx_velocity_average`].
#[must_use]
pub fn wx_velocity_sum(nodes: &[[f32; 3]]) -> ([f64; 3], f32) {
    let f = f64::from;
    let mut s = [0.0f64; 3];
    for n in nodes {
        s[0] += f(n[0]);
        s[1] += f(n[1]);
        s[2] += f(n[2]);
    }
    (s, s[2] as f32)
}
