//! The camera's per-frame driver (`systems/uicamera/driver.md`): smoothing a unit's facing, placing
//! the eye, and the six keyboard axes.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`; `f64::sin_cos` is `fsincos`.

const PI: f64 = f32::from_bits(0x4049_0fdb) as f64; // 0x80891c
const TWO_PI: f64 = f32::from_bits(0x40c9_0fdb) as f64; // 0x80295c
/// 0.09 at `0x808a28`: the smallest facing step, and the error below which the new facing is used.
const DEADZONE: f64 = f32::from_bits(0x3db8_51ec) as f64;
/// 15 degrees at `0x808a2c`: the largest facing step.
const MAX_STEP: f64 = f32::from_bits(0x3e86_0a92) as f64;
const PITCH_LO: f64 = f32::from_bits(0xbfc6_d3f2) as f64; // -89 degrees, 0x8089dc
const PITCH_HI: f64 = f32::from_bits(0x3fc6_d3f2) as f64; // +89 degrees, 0x8089d8
/// 0.001 at `0x801360`: seconds per millisecond tick.
const DT: f64 = f32::from_bits(0x3a83_126f) as f64;
const DEG2RAD: f64 = f32::from_bits(0x3c8e_fa35) as f64; // 0x7ffaac
/// 0.00025 at `0x808ac8`: the pitch axis's seconds-and-quarter-degree factor.
const PITCH_DT: f64 = f32::from_bits(0x3983_126f) as f64;
const ZOOM_LO: f64 = 0.0; // 0x8089a8
const ZOOM_HI: f64 = f32::from_bits(0x4248_0000) as f64; // 50.0, 0x8089a4

/// `CCamera::FilterFacing` (`0x50d810`), reading the last used facing at `+0x184` and storing
/// nothing: returns the facing the camera follows (`driver#smoothing-the-facing`), at register
/// precision, as the caller uses it.
///
/// Without `enabled` (`camerasmooth` set and the unit moved) it returns `target`. Otherwise
/// `d = target - field`, unwrapped once by 2π into `[-π, π]` and rounded to `f32`. An error of 0.09
/// or less returns `target`; up to 15 degrees the step is 0.09, beyond that 15 degrees. The step
/// takes the sign of `d`, and `field + step` is wrapped once into `[0, 2π]`.
pub fn filter_facing(field: f32, target: f32, enabled: bool) -> f64 {
    if !enabled {
        return f64::from(target);
    }
    let mut d = f64::from(target) - f64::from(field);
    if d <= PI {
        if d < -PI {
            d += TWO_PI;
        }
    } else {
        d -= TWO_PI;
    }
    let df = d as f32;
    let adf = f64::from(df).abs();
    let step = if adf <= MAX_STEP {
        if adf <= DEADZONE {
            return f64::from(target);
        }
        DEADZONE
    } else {
        MAX_STEP
    };
    let signed = if f64::from(df) < 0.0 { -step } else { step };
    let r = signed + f64::from(field);
    if r < 0.0 {
        return r + TWO_PI;
    }
    if TWO_PI < r {
        return r - TWO_PI;
    }
    r
}

/// `CCamera::PlaceEye` (`0x50de00`): `out[i] = (pivot[i] - distance * dir[i]) + offset[i]`, where
/// `dir` is the forward row from the camera's vtable (`driver#placing-the-eye`). The three axes
/// round differently, as the binary stores:
///
/// - x: the difference `pivot.x - distance * dir.x` is stored as `f32` (`0x50de3f`) before the
///   offset is added; the product stays at register precision.
/// - y: nothing is stored between the product and the sum; the whole chain stays at register
///   precision.
/// - z: the product `distance * dir.z` is stored as `f32` (`0x50de37`); the difference stays at
///   register precision.
///
/// Each sum is stored as `f32`.
pub fn place_eye(pivot: [f32; 3], dir: [f32; 3], distance: f32, offset: [f32; 3]) -> [f32; 3] {
    let t = f64::from(distance);
    let xsub = (f64::from(pivot[0]) - t * f64::from(dir[0])) as f32;
    let ox = (f64::from(xsub) + f64::from(offset[0])) as f32;
    let oy = ((f64::from(pivot[1]) - t * f64::from(dir[1])) + f64::from(offset[1])) as f32;
    let zprod = (t * f64::from(dir[2])) as f32;
    let oz = ((f64::from(pivot[2]) - f64::from(zprod)) + f64::from(offset[2])) as f32;
    [ox, oy, oz]
}

/// The words `CCamera::IntegrateAxes` reads and writes (`driver#the-keyboard-axes`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KbState {
    /// The mode word, `+0x90`: bits 0x4000000, 0x1000000 and 0x2000000 stop the live distance, yaw
    /// and pitch from following their targets.
    pub mode: u32,
    /// The axis word, `+0x128`: bit 2i says axis i is active and bit 2i+1 that it is stopping.
    pub axes: u32,
    /// The axes' start times, `+0x12c + 4i`.
    pub start_ms: [i32; 6],
    /// The axes' end times, `+0x144 + 4i`.
    pub end_ms: [i32; 6],
    /// The axes' queued ends, `+0x15c + 4i`; 0 means the stop helper `0x50fd90` is not called.
    pub queued_end: [i32; 6],
    /// The distance target, `+0x198`.
    pub distance: f32,
    /// The live distance, `+0xec`.
    pub live_distance: f32,
    /// The yaw target, `+0x210`.
    pub yaw: f32,
    /// The live yaw, `+0xf0`.
    pub live_yaw: f32,
    /// The pitch target, `+0x1e0`.
    pub pitch: f32,
    /// The live pitch, `+0xf4`.
    pub live_pitch: f32,
}

/// `CCamera::IntegrateAxes` (`0x5112d0`): advances the targets of the six axes (axis 0 lowers the
/// distance target, 1 raises it, 2 raises the yaw target, 3 lowers it, 4 raises the pitch target,
/// 5 lowers it) by their speed CVars over the elapsed milliseconds.
///
/// It sets mode bit 0x40. The distance is kept between 0 and `clamp(max * factor, 0, 50)`. For each
/// axis whose active bit is set: with the stopping bit clear it takes `n = now - start_ms[i]`, skips
/// the axis when that is negative and stores `now` as the new start; with it set it takes
/// `n = max(end_ms[i] - start_ms[i], 0)` and clears both bits. The distance moves by
/// `rate * n * 0.001`, the yaw by `rate * n * 0.001 * π/180`, wrapped once by 2π, and the pitch by
/// `rate * n * 0.00025 * π/180`, clamped to ±89 degrees. Each target is stored as `f32`, and the
/// live word follows it unless its glide bit is set in the mode word.
///
/// A queued end that has arrived (`now - queued_end[i] >= 0`) first calls `CCamera::StopAxis`
/// (`0x50fd90`), which stores `now` as the axis's end time and sets its stopping bit; this
/// transcribes the path with every queued end 0, where that call is skipped.
pub fn integrate_axes(
    st: &mut KbState,
    cvar_zoom_a: f32,
    cvar_zoom_b: f32,
    cvar_zoom_rate: f32,
    cvar_yaw_rate: f32,
    cvar_pitch_rate: f32,
    now: i32,
) {
    let zoom_lo = ZOOM_LO;
    let prod = f64::from(cvar_zoom_a) * f64::from(cvar_zoom_b);
    let zoom_hi = if ZOOM_LO <= prod {
        if ZOOM_HI <= prod {
            ZOOM_HI
        } else {
            prod
        }
    } else {
        ZOOM_LO
    };
    st.mode |= 0x40;

    for axis in 0..6usize {
        let pair = (axis * 2) as u32;
        if st.axes & (1u32 << pair) == 0 {
            continue;
        }
        let elapsed: i32;
        if st.axes & (1u32 << (pair + 1)) == 0 {
            let d = now.wrapping_sub(st.start_ms[axis]);
            if d < 0 {
                continue;
            }
            elapsed = d;
            st.start_ms[axis] = now;
        } else {
            let d = st.end_ms[axis].wrapping_sub(st.start_ms[axis]);
            elapsed = if d < 0 { 0 } else { d };
            st.axes &= !(3u32 << pair);
        }
        let n = f64::from(elapsed); // fimul, a signed int32
        match axis {
            0 => {
                let s = f64::from(st.distance) - f64::from(cvar_zoom_rate) * n * DT;
                st.distance = s as f32;
                if s < zoom_lo {
                    st.distance = zoom_lo as f32;
                }
                if st.mode & 0x4000000 == 0 {
                    st.live_distance = st.distance;
                }
            }
            1 => {
                let s = f64::from(st.distance) + f64::from(cvar_zoom_rate) * n * DT;
                st.distance = s as f32;
                if zoom_hi < s {
                    st.distance = zoom_hi as f32;
                }
                if st.mode & 0x4000000 == 0 {
                    st.live_distance = st.distance;
                }
            }
            2 => {
                let s = f64::from(cvar_yaw_rate) * n * DT * DEG2RAD + f64::from(st.yaw);
                st.yaw = if TWO_PI < s {
                    (s - TWO_PI) as f32
                } else {
                    s as f32
                };
                if st.mode & 0x1000000 == 0 {
                    st.live_yaw = st.yaw;
                }
            }
            3 => {
                let s = f64::from(st.yaw) - f64::from(cvar_yaw_rate) * n * DT * DEG2RAD;
                st.yaw = if s < 0.0 {
                    (s + TWO_PI) as f32
                } else {
                    s as f32
                };
                if st.mode & 0x1000000 == 0 {
                    st.live_yaw = st.yaw;
                }
            }
            4 => {
                let s = f64::from(cvar_pitch_rate) * n * PITCH_DT * DEG2RAD + f64::from(st.pitch);
                st.pitch = if PITCH_HI < s {
                    PITCH_HI as f32
                } else {
                    s as f32
                };
                if st.mode & 0x2000000 == 0 {
                    st.live_pitch = st.pitch;
                }
            }
            _ => {
                let s = f64::from(st.pitch) - f64::from(cvar_pitch_rate) * n * PITCH_DT * DEG2RAD;
                st.pitch = if s < PITCH_LO {
                    PITCH_LO as f32
                } else {
                    s as f32
                };
                if st.mode & 0x2000000 == 0 {
                    st.live_pitch = st.pitch;
                }
            }
        }
    }
}
