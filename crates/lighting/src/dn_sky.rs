//! The colour and body kernels of the sky: `PackColourBytes` (`0x6d5270`,
//! `systems/lighting/sky.md#sky-colours`), `LerpColourBytes` (`0x6d51e0`,
//! `systems/lighting/fog.md#light-colours`), `UpdateSunDirection` (`0x6d3a10`,
//! `systems/lighting/celestial.md#sun-direction`), the ambient darken inside `BuildLightColours`
//! (`0x6d37e0`), `UpdateCelestialBodies` (`0x6d3b80`, `celestial.md#celestial-bodies`) and
//! `BuildCloudDome` (`0x6d0530`, `systems/lighting/clouds.md#cloud-dome`). The x87 runs at 53 bits
//! and stores `f32` at each `fstp dword`; `fsin`, `fcos` and `floor` are the host's. The light
//! state is passed in and the writes returned, as the exe's globals are not modelled.

use crate::dn_bands::dn_track_eval;
use crate::x87::fistp;

/// `0x6d5270` (`ecx` = out, four bytes on the stack, `ret 0x10`): `(a << 24) | (b << 16) | (c << 8)
/// |
/// d`, each argument's low byte.
pub fn dn_pack_bytes_color(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
}

/// `0x6d51e0(a, &src)` (`ecx` = &dst, `ret 8`): at a weight of exactly 0xff the whole `src` dword
/// is copied. Otherwise bytes 0 to 2 become `dst + (((src − dst)·a) >> 8)`, the signed product
/// shifted logically and the byte add wrapping, and byte 3 is 0.
pub fn dn_channel_lerp8(dst: u32, src: u32, a: u32) -> u32 {
    if a == 0xff {
        return src;
    }
    let d0 = dst & 0xff;
    let d1 = (dst >> 8) & 0xff;
    let d2 = (dst >> 16) & 0xff;
    let s0 = src & 0xff;
    let s1 = (src >> 8) & 0xff;
    let s2 = (src >> 16) & 0xff;
    let lerp = |d: u32, s: u32| -> u32 {
        let prod = (s.wrapping_sub(d)).wrapping_mul(a);
        (prod >> 8).wrapping_add(d) & 0xff
    };
    let r2 = lerp(d2, s2);
    let r1 = lerp(d1, s1);
    let r0 = lerp(d0, s0);
    (r2 << 16) | (r1 << 8) | r0
}

/// π as an `f32`, `[0x811508]` (`0x40490fdb`).
const PI_F32: f32 = std::f32::consts::PI;

/// The sun's azimuth and elevation tracks, which `0x6d3a10` builds once: keys 0, 0.25, 0.5 and
/// 0.75; azimuth alternates π·0.7055 (`[0x81163c]`) and π·0.6111 (`[0x811638]`), elevation is
/// π·1.25 (`[0x80c5d0]`) at all four, each product stored as `f32`.
fn sun_az_track() -> [[f32; 2]; 4] {
    let a = (f64::from(PI_F32) * f64::from(f32::from_bits(0x3f34_9f4a))) as f32;
    let b = (f64::from(PI_F32) * f64::from(f32::from_bits(0x3f1c_71c7))) as f32;
    [[0.0, a], [0.25, b], [0.5, a], [0.75, b]]
}
fn sun_el_track() -> [[f32; 2]; 4] {
    let e = (f64::from(PI_F32) * f64::from(f32::from_bits(0x3fa0_0000))) as f32;
    [[0.0, e], [0.25, e], [0.5, e], [0.75, e]]
}

/// `0x6d3a10()`: the sun direction from the time of day `[0xce9b64]`. The azimuth `az` is the track
/// sample stored as `f32` (`0x6d3b17`); the elevation `el` stays in the register. The components
/// stored at `0xce9cc8`, `0xce9ccc` and `0xce9cd0` are `sin(az)·cos(el)`, `sin(az)·sin(el)` and
/// `cos(az)`, each as `f32`.
pub fn dn_sun_dir(time_of_day: f32) -> [f32; 3] {
    let az = dn_track_eval(&sun_az_track(), 4, time_of_day);
    let el = dn_track_eval(&sun_el_track(), 4, time_of_day);
    let az = f64::from(az as f32);
    let x = (az.sin() * el.cos()) as f32;
    let y = (az.sin() * el.sin()) as f32;
    let z = (az.cos()) as f32;
    [x, y, z]
}

/// The ambient darken `BuildLightColours` (`0x6d37e0`) applies to each channel: `((c + 3)·0x55) >>
/// 8` (`add eax,3; imul eax,eax,0x55; shr eax,8`, at `0x6d38c2`, `0x6d3902` and `0x6d3912`).
pub fn dn_ambient_darken_channel(c: u8) -> u8 {
    (((c as u32 + 3) * 0x55) >> 8) as u8
}

/// The field `[0xce9cf4]` of `BuildLightColours` (`0x6d38da` to `0x6d3945`): the darkened low three
/// bytes of the colour at `0xce9c10`, with the byte `[0xce9c5a]` on top. `dn_color` runs the rest.
pub fn dn_fog_ambient_darkened(base_color: u32, alpha: u8) -> u32 {
    let b0 = (base_color & 0xff) as u8;
    let b1 = ((base_color >> 8) & 0xff) as u8;
    let b2 = ((base_color >> 16) & 0xff) as u8;
    ((alpha as u32) << 24)
        | ((dn_ambient_darken_channel(b2) as u32) << 16)
        | ((dn_ambient_darken_channel(b1) as u32) << 8)
        | (dn_ambient_darken_channel(b0) as u32)
}

/// `[0x811508]`, π.
const C_PI: f32 = f32::from_bits(0x4049_0fdb);
/// `[0x811510]`, 0.229166… (the first day-period key).
const C_DAY0: f32 = f32::from_bits(0x3e6a_aaab);
/// `[0x811514]`, 0.5.
const C_DAY1: f32 = f32::from_bits(0x3f00_0000);
/// `[0x811518]`, 0.895833….
const C_DAY2: f32 = f32::from_bits(0x3f65_5555);
/// `[0x81151c]`, 0.916666….
const C_DAY3: f32 = f32::from_bits(0x3f6a_aaab);
/// `[0x811520]`, 1.0.
const C_DAY4: f32 = f32::from_bits(0x3f80_0000);
/// `[0x811524]`, 0.0.
const C_DAY5: f32 = f32::from_bits(0x0000_0000);
/// `[0x811528]`, 0.166666….
const C_DAY6: f32 = f32::from_bits(0x3e2a_aaab);
/// `[0x811644]`, 0.916666… (el-scale 0.9167·PI).
const C_0_9167: f32 = f32::from_bits(0x3f6a_aaab);
/// `[0x811648]`, 0.194444….
const C_0_1944: f32 = f32::from_bits(0x3e47_1c72);
/// `[0x81164c]`, 0.003472… (the ±ε key offset).
const C_EPS: f32 = f32::from_bits(0x3b63_8e39);
/// `[0x811650]`, 0.555555….
const C_0_5556: f32 = f32::from_bits(0x3f0e_38e4);
/// `[0x81161c]`, 0.020833….
const C_0_0208: f32 = f32::from_bits(0x3caa_aaab);
/// `[0x811620]`, 0.082638….
const C_0_0826: f32 = f32::from_bits(0x3da9_3e94);
/// `[0x811640]`, 0.052083….
const C_0_0521: f32 = f32::from_bits(0x3d55_5555);
/// `[0x7ff9d0]`, 0.027777….
const C_0_0278: f32 = f32::from_bits(0x3ce3_8e39);
/// `[0x7ff9d8]`, 1.0 (the day-period inversion `1 - frac`).
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x802978]`, 0.125.
const C_0_125: f32 = f32::from_bits(0x3e00_0000);
/// `[0x8029b0]`, 0.25.
const C_0_25: f32 = f32::from_bits(0x3e80_0000);
/// `[0x8012cc]`, 0.75.
const C_0_75: f32 = f32::from_bits(0x3f40_0000);
/// `[0x810018]`, 0.833333….
const C_0_8333: f32 = f32::from_bits(0x3f55_5555);
/// `[0x80c4e8]`, 12.0 (the celestial sphere radius).
const C_RADIUS: f32 = f32::from_bits(0x4140_0000);
/// The immediate 2.0.
const C_2: f32 = f32::from_bits(0x4000_0000);
/// The immediate 1.5.
const C_1_5: f32 = f32::from_bits(0x3fc0_0000);
/// The literal stored as the fifth key of the moon and fourth-body azimuth tracks.
const C_3F7F1C72: f32 = f32::from_bits(0x3f7f_1c72);
/// `[0x80a000]`, 65536.0.
const C_65536: f32 = f32::from_bits(0x4780_0000);
/// `[0x86f2c8]`, 0.5 (the round-bias before `fistp`).
const C_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x80a8b0]`, 1/65536.
const C_INV65536: f32 = f32::from_bits(0x3780_0000);

/// The eight tracks `0x6d3b80` builds once (key, value): each value is a product of π or a
/// constant, stored as `f32`, with keys from the day-period constants.
struct CelestialTables {
    sun_az: [[f32; 2]; 5],
    sun_el: [[f32; 2]; 3],
    moon_az: [[f32; 2]; 5],
    moon_el: [[f32; 2]; 3],
    body4_az: [[f32; 2]; 5],
    body4_el: [[f32; 2]; 3],
    sun_scale: [[f32; 2]; 4],
    body24_scale: [[f32; 2]; 4],
}

fn celestial_tables() -> CelestialTables {
    let pi = |k: f32| (f64::from(C_PI) * f64::from(k)) as f32;
    let az_a = pi(C_0_5556);
    let az_b = pi(C_0_0278);
    let az_c = pi(C_0_1944);
    let az_d = pi(C_0_5556);
    let el_sun = pi(C_0_25);
    let el_moon = pi(C_0_25);
    CelestialTables {
        // `0xce8d64`, five entries: the sun's azimuth.
        sun_az: [
            [C_DAY0, az_a],
            [C_DAY1 - C_EPS, az_b],
            [C_DAY1, az_b],
            [C_DAY1 + C_EPS, az_b],
            [C_DAY2, az_a],
        ],
        // `0xce8d4c`, three: the sun's elevation.
        sun_el: [[C_DAY0, el_sun], [C_DAY1, el_sun], [C_DAY2, el_sun]],
        // `0xce8d24`, five: the moon's azimuth.
        moon_az: [
            [C_DAY5, az_c],
            [C_DAY5 + C_EPS, az_c],
            [C_DAY6, az_d],
            [C_DAY3, az_d],
            [C_3F7F1C72, az_c],
        ],
        // `0xce8d0c`, three: the moon's elevation.
        moon_el: [[C_DAY5, el_moon], [C_DAY6, el_moon], [C_DAY3, el_moon]],
        // `0xce8ce4`, five: the fourth body's azimuth, the moon's layout.
        body4_az: [
            [C_DAY5, az_c],
            [C_DAY5 + C_EPS, az_c],
            [C_DAY6, az_d],
            [C_DAY3, az_d],
            [C_3F7F1C72, az_c],
        ],
        // `0xce8ccc`, three: its elevation, π·0.75, π·0.8333 and π·0.9167.
        body4_el: [
            [C_DAY5, pi(C_0_75)],
            [C_DAY6, pi(C_0_8333)],
            [C_DAY3, pi(C_0_9167)],
        ],
        // `0xce8cac`, four: the sun's scale, 2, 1, 1, 2.
        sun_scale: [
            [C_DAY0 + C_0_0208, C_2],
            [C_DAY0 + C_0_0521, C_ONE],
            [C_DAY2 - C_0_0521, C_ONE],
            [C_DAY2 - C_0_0208, C_2],
        ],
        // `0xce8c8c`, four: the scale of the moon and the fourth body, 1, 1.5, 1.5, 1.
        body24_scale: [
            [C_DAY6 - C_0_125, C_ONE],
            [C_DAY6, C_1_5],
            [C_DAY3, C_1_5],
            [C_DAY3 + C_0_0826, C_ONE],
        ],
    }
}

/// A body's world position (bytes `0x6d3ff5` to `0x6d4085`). The azimuth track sample is stored as
/// `f32` and the elevation stays in the register. `x = cos(el)·sin(az)` is stored as `f32`, `y =
/// sin(el)·sin(az)` and `z = cos(az)` stay wide; the magnitude is `√((x² + y²) + z²)` with `x` as
/// stored, the scale is `12/magnitude`, and each component times it plus the camera offset is
/// stored as `f32`, with `x` and `y` rounded to `f32` before the offset is added.
fn celestial_body(
    az_track: &[[f32; 2]],
    az_n: i32,
    el_track: &[[f32; 2]],
    el_n: i32,
    phase: f32,
    off: [f32; 3],
) -> [f32; 3] {
    let az = dn_track_eval(az_track, az_n, phase) as f32;
    let el = dn_track_eval(el_track, el_n, phase);
    let az = f64::from(az);
    let sinaz = az.sin();
    let x_f32 = (el.cos() * sinaz) as f32;
    let y = el.sin() * sinaz;
    let z = az.cos();
    let xf = f64::from(x_f32);
    let mag = (xf * xf + y * y + z * z).sqrt();
    let s = f64::from(C_RADIUS) / mag;
    let px = ((xf * s) as f32 as f64 + f64::from(off[0])) as f32;
    let py = ((y * s) as f32 as f64 + f64::from(off[1])) as f32;
    let pz = (z * s + f64::from(off[2])) as f32;
    [px, py, pz]
}

/// What `UpdateCelestialBodies` writes: the positions and scales of the sun, the moon and the
/// fourth body, their copies, and the day blend.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CelestialOut {
    /// `0xce9858`, `0xce985c`, `0xce9860`.
    pub sun_pos: [f32; 3],
    /// `0xce97b4`, `0xce97b8`, `0xce97bc`.
    pub sun_pos_copy: [f32; 3],
    /// `0xce986c`.
    pub sun_scale: f32,
    /// `0xce9878`, `0xce987c`, `0xce9880`.
    pub moon_pos: [f32; 3],
    /// `0xce9704`, `0xce9708`, `0xce970c`.
    pub moon_pos_copy: [f32; 3],
    /// `0xce988c`.
    pub moon_scale: f32,
    /// `0xce9788`.
    pub moon_scale_copy_a: f32,
    /// `0xce978c`.
    pub moon_scale_copy_b: f32,
    /// `0xce9898`, `0xce989c`, `0xce98a0`.
    pub body4_pos: [f32; 3],
    /// `0xce98ac`.
    pub body4_scale: f32,
    /// `0xce9d08`.
    pub day_blend: f32,
}

/// The globals `UpdateCelestialBodies` reads.
#[derive(Clone, Copy, Debug)]
pub struct CelestialIn {
    /// The camera, `0xce9b78`, `0xce9b7c`, `0xce9b80`.
    pub offset: [f32; 3],
    /// `0xce9b64`.
    pub time_of_day: f32,
    /// `0xce9b68`.
    pub moon_epoch: f32,
    /// `0xce9870`.
    pub sun_scale_mul: f32,
    /// `0xce9890`.
    pub moon_scale_mul: f32,
    /// `0xce98b0`.
    pub body4_scale_mul: f32,
    /// `0xce98b4`, the divisor of the moon phase.
    pub day_length: f32,
}

/// `0x6d3b80()`: three bodies are placed at the time of day, each by `celestial_body` with its
/// tracks, and scaled by their scale track times a multiplier (the sun's track at `0xce8cac`, the
/// moon's and the fourth body's at `0xce8c8c`). The fourth body is placed at a moon phase: each of
/// `epoch·65536` and `time·65536` is stored as `f32`, minus 0.5, `fistp` (so an exact odd integer
/// comes out one low); their sum `esi`, and `eax` the same for `day_length·floor((epoch + time) /
/// day_length)`, taken as the `f32` `day_length·floor·65536`, minus 0.5, `fistp`. When `eax` is not
/// below `esi` (unsigned, `cmp eax,esi; jb`) it becomes `esi`. The phase is `f32((esi − eax)
/// ·(1/65536) / day_length)` with the difference read as an unsigned 32-bit integer (`fild qword`
/// with a zero high dword). The day blend is `day_period_blend`.
pub fn dn_celestial_positions(input: CelestialIn) -> CelestialOut {
    let t = celestial_tables();
    let off = input.offset;
    let tod = input.time_of_day;
    let mut out = CelestialOut::default();

    out.sun_pos = celestial_body(&t.sun_az, 5, &t.sun_el, 3, tod, off);
    out.sun_pos_copy = out.sun_pos;
    let scale = dn_track_eval(&t.sun_scale, 4, tod);
    out.sun_scale = (scale * f64::from(input.sun_scale_mul)) as f32;

    out.moon_pos = celestial_body(&t.moon_az, 5, &t.moon_el, 3, tod, off);
    out.moon_pos_copy = out.moon_pos;
    let scale = dn_track_eval(&t.body24_scale, 4, tod) * f64::from(input.moon_scale_mul);
    out.moon_scale = scale as f32;
    out.moon_scale_copy_a = scale as f32;
    out.moon_scale_copy_b = scale as f32;

    let day_index = |v: f32| -> i32 {
        let scaled = (f64::from(v) * f64::from(C_65536)) as f32;
        fistp(f64::from(scaled) - f64::from(C_HALF))
    };
    let epoch_idx = day_index(input.moon_epoch);
    let tod_idx = day_index(tod);
    let esi = tod_idx.wrapping_add(epoch_idx);
    let floor_arg = (f64::from(input.moon_epoch) + f64::from(tod)) / f64::from(input.day_length);
    let moonf = floor_arg.floor();
    let rescaled = (f64::from(input.day_length) * moonf * f64::from(C_65536)) as f32;
    let mut eax = fistp(f64::from(rescaled) - f64::from(C_HALF));
    if (eax as u32) >= (esi as u32) {
        eax = esi;
    }
    let diff = esi.wrapping_sub(eax) as u32;
    let moon_phase = (f64::from(diff) * f64::from(C_INV65536) / f64::from(input.day_length)) as f32;
    out.body4_pos = celestial_body(&t.body4_az, 5, &t.body4_el, 3, moon_phase, off);
    let scale = dn_track_eval(&t.body24_scale, 4, moon_phase);
    out.body4_scale = (scale * f64::from(input.body4_scale_mul)) as f32;

    out.day_blend = day_period_blend(tod);
    out
}

/// The day blend `[0xce9d08]` (bytes `0x6d432f` to `0x6d4470`): four segments of the time of day,
/// each tested as `lo <= tod` (`fcomp`, `test ah,1; jne`) and `tod < hi` (`test ah,5; jp`), both
/// false for NaN. In `[0.229, 0.5)` the position `(tod − lo)/(hi − lo)`; in `[0.5, 0.896)` one
/// minus it; in `[0.9167, 1.0)` the position; in `[0, 0.1667)` one minus it; otherwise 0. The ratio
/// is taken at 53 bits and stored once as `f32`.
#[allow(clippy::manual_range_contains)]
fn day_period_blend(tod: f32) -> f32 {
    let ramp = |lo: f32, hi: f32| -> f64 {
        (f64::from(tod) - f64::from(lo)) / (f64::from(hi) - f64::from(lo))
    };
    if C_DAY0 <= tod && tod < C_DAY1 {
        ramp(C_DAY0, C_DAY1) as f32
    } else if C_DAY1 <= tod && tod < C_DAY2 {
        (f64::from(C_ONE) - ramp(C_DAY1, C_DAY2)) as f32
    } else if C_DAY3 <= tod && tod < C_DAY4 {
        ramp(C_DAY3, C_DAY4) as f32
    } else if C_DAY5 <= tod && tod < C_DAY6 {
        (f64::from(C_ONE) - ramp(C_DAY5, C_DAY6)) as f32
    } else {
        0.0
    }
}

/// The twelve ring fractions at `[0x811570]`, 0 to 0.25 (a quarter turn), bunching near the rim;
/// each times π is the ring's angle.
const TRIG_RING_FRAC: [f32; 12] = [
    f32::from_bits(0x0000_0000),
    f32::from_bits(0x3ccc_cccd),
    f32::from_bits(0x3d4c_cccd),
    f32::from_bits(0x3d99_999a),
    f32::from_bits(0x3dcc_cccd),
    f32::from_bits(0x3e00_0000),
    f32::from_bits(0x3e19_999a),
    f32::from_bits(0x3e33_3333),
    f32::from_bits(0x3e51_eb85),
    f32::from_bits(0x3e6b_851f),
    f32::from_bits(0x3e7a_e148),
    f32::from_bits(0x3e80_0000),
];

/// The vertex alpha of each ring, bytes at `[0x8115a0]`: opaque for the inner nine, 0x80, then two
/// transparent rim rings. The colour is `(alpha << 24) | 0xffffff`.
const TRIG_RING_ALPHA: [u8; 12] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x80, 0x00, 0x00,
];

/// `[0x81160c]`, 0.09090909.
const C_INV12: f32 = f32::from_bits(0x3dba_2e8c);
/// `[0x80295c]`, 2π.
const C_TWOPI: f32 = f32::from_bits(0x40c9_0fdb);
/// `[0x8026bc]`, 9.5367e-7: the angle below which a ring is a point.
const C_EPS_TRIG: f32 = f32::from_bits(0x3580_0000);
/// `[0x802998]`, 0.0625.
const C_0_0625: f32 = f32::from_bits(0x3d80_0000);
/// `[0x7ffa24]`, 0.5.
const C_HALF_UV: f32 = f32::from_bits(0x3f00_0000);

/// The four vectors `BuildCloudDome` fills in the dome object at `+0x6c`: 192 positions, 192
/// texture coordinates, 192 colours and 374 `u16` indices. `vertex_count` (`+0x9e`) counts the
/// vertices written and `index_count` (`+0x9c`) is 0x176.
#[derive(Clone, Debug, PartialEq)]
pub struct SunTrigMesh {
    pub pos: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub color: Vec<u32>,
    pub index: Vec<u16>,
    pub vertex_count: u16,
    pub index_count: u16,
}

/// `n mod 16` with the sign fix (`and 0x8000000f; jns; dec; or 0xfffffff0; inc`): for the
/// non-negative `n` of the loop, `n & 0xf`, so 16 gives 0.
fn mod16_signfix(n: i32) -> i32 {
    let mut m = n & (0x8000_000fu32 as i32);
    if m < 0 {
        m = ((m - 1) | (0xffff_fff0u32 as i32)) + 1;
    }
    m
}

/// `0x6d0530(radius)` (`ecx` = the dome, `ret 4`): twelve rings of up to sixteen vertices. A ring's
/// angle `θ = π·frac` is held at 53 bits; `sin θ` and the texture radius `ring·(1/12)·0.5` are
/// stored as `f32`, `cos θ` stays wide. The vertex is `(sin θ·sin az·r, cos az·sin θ·r, cos θ·r −
/// cos(π/4))` stored as `f32`, with the texture coordinate `(sin az·v + 0.5, cos az·v + 0.5)`; `az`
/// starts at 0 and steps by `2π·0.0625`, stored as `f32`. A ring stops after one vertex when `|θ as
/// f32| < ε` or `|θ − 2π| < ε` (`test ah,5; jnp`, which exits only on a value below ε), and after
/// sixteen. Rings after the first add 17 index pairs: the previous ring's `n mod 16` offset by its
/// start, or 0 when its angle was below ε, and this ring's `n mod 16` offset by its start, or 0
/// when it is within ε of 2π.
pub fn dn_sun_trig_table(radius: f32) -> SunTrigMesh {
    let r = f64::from(radius);
    let pole_z = -((f64::from(C_PI) * f64::from(C_0_25)).cos() as f32);
    let az_step = ((f64::from(C_PI) + f64::from(C_PI)) * f64::from(C_0_0625)) as f32;

    let mut pos: Vec<[f32; 3]> = Vec::with_capacity(0xc0);
    let mut uv: Vec<[f32; 2]> = Vec::with_capacity(0xc0);
    let mut color: Vec<u32> = Vec::with_capacity(0xc0);
    let mut index: Vec<u16> = Vec::with_capacity(0x176);

    let mut prev_ring_start: i32 = 0;
    let mut prev_ring_theta: f32 = 0.0;

    for ring in 0..12usize {
        let ring_theta = f64::from(C_PI) * f64::from(TRIG_RING_FRAC[ring]);
        let cos_ring = ring_theta.cos();
        let sin_ring = ring_theta.sin() as f32;
        let ring_start = pos.len() as i32;
        let v_coord = (f64::from(ring as i32) * f64::from(C_INV12) * f64::from(C_HALF_UV)) as f32;

        let mut az = 0.0f32;
        let mut step = 0i32;
        loop {
            let cos_az = f64::from(az).cos();
            let sin_az = f64::from(az).sin();
            let px = (f64::from(sin_ring) * sin_az * r) as f32;
            let py = (cos_az * f64::from(sin_ring) * r) as f32;
            let pz = (cos_ring * r + f64::from(pole_z)) as f32;
            pos.push([px, py, pz]);
            color.push(((TRIG_RING_ALPHA[ring] as u32) << 24) | 0x00ff_ffff);
            let u = (sin_az * f64::from(v_coord) + f64::from(C_HALF_UV)) as f32;
            let vv = (cos_az * f64::from(v_coord) + f64::from(C_HALF_UV)) as f32;
            uv.push([u, vv]);

            if (ring_theta as f32).abs() < C_EPS_TRIG
                || (ring_theta - f64::from(C_TWOPI)).abs() < f64::from(C_EPS_TRIG)
            {
                break;
            }
            az = (f64::from(az) + f64::from(az_step)) as f32;
            step += 1;
            if step >= 0x10 {
                break;
            }
        }

        if ring != 0 {
            for n in 0..0x11i32 {
                let a = if f64::from(prev_ring_theta).abs() >= f64::from(C_EPS_TRIG) {
                    mod16_signfix(n)
                } else {
                    0
                };
                let b = if (ring_theta - f64::from(C_TWOPI)).abs() >= f64::from(C_EPS_TRIG) {
                    mod16_signfix(n)
                } else {
                    0
                };
                index.push((a + prev_ring_start) as u16);
                index.push((b + ring_start) as u16);
            }
        }

        prev_ring_theta = ring_theta as f32;
        prev_ring_start = ring_start;
    }

    let vertex_count = pos.len() as u16;
    SunTrigMesh {
        pos,
        uv,
        color,
        index,
        vertex_count,
        index_count: 0x176,
    }
}
