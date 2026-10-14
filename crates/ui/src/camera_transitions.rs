//! The camera's transitions (`systems/uicamera/transitions.md`): the five angle channels' arming
//! setters, each an outer function (`Arm*`) that decides whether a request is new and what it costs
//! and an inner one (`Begin*Transition`) that writes the descriptor; and the integrators
//! `ShiftPitch` and `ShiftYaw`, which add one delta to a channel's target, start and live values.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and an `fst`/`fstp dword` is an `as f32`.
//! `__ftol` truncates toward zero, as `as i32` does.

// ---- the setters ----

/// π, the `f32` at `0x80891c`: the wrap loop's bound; the lower loop uses its exact negation.
const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
/// 2π, the `f32` at `0x80295c`: the wrap loop's step.
const TWO_PI: f64 = f32::from_bits(0x40c9_0fdb) as f64;
/// 0.001, the `f32` at `0x801360`: how close to the target counts as already there.
const DT: f64 = f32::from_bits(0x3a83_126f) as f64;
/// π/180, the `f32` at `0x7ffaac`: the speed CVar's degrees to radians.
const PI_OVER_180: f64 = f32::from_bits(0x3c8e_fa35) as f64;
/// -1000, the `f32` at `0x801448`: the delay's multiplier before `__ftol`.
const NEG_1000: f64 = f32::from_bits(0xc47a_0000) as f64;

/// The mode-word (`+0x90`) bit of the field-of-view transition.
pub const ARM_FOV: u32 = 0x4000_0000;
/// The mode-word bit of the ground-pitch transition.
pub const ARM_GROUND_PITCH: u32 = 0x1000_0000;
/// The mode-word bit of the pitch transition.
pub const ARM_PITCH: u32 = 0x0200_0000;
/// The mode-word bit of the pivot-bias transition.
pub const ARM_PIVOT_BIAS: u32 = 0x0800_0000;
/// The mode-word bit of the yaw transition.
pub const ARM_YAW: u32 = 0x0100_0000;

/// The words of one channel the setters read and write, as raw dwords so a test compares bits.
/// A channel is a live word, the mode word and the six dwords of its descriptor
/// (`systems/uicamera/transitions.md#the-channels`); the live words are `+0x10c` (field of view),
/// `+0x108` (ground pitch), `+0xf4` (pitch), `+0x104` (pivot bias) and `+0xf0` (yaw), and the
/// descriptors start at `+0x220`, `+0x1a8`, `+0x1d8`, `+0x1f0` and `+0x208`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CamState {
    /// The live angle, `f32` bits.
    pub live: u32,
    /// The mode word (`+0x90`).
    pub flags: u32,
    /// Descriptor `+0x00`: the start time in milliseconds, an integer.
    pub start_ms: u32,
    /// Descriptor `+0x04`: the duration in seconds, `f32` bits.
    pub duration: u32,
    /// Descriptor `+0x08`: the target, `f32` bits.
    pub target: u32,
    /// Descriptor `+0x0c`: the angle the transition runs from, the live angle wrapped toward the
    /// target, `f32` bits.
    pub from: u32,
    /// Descriptor `+0x10`: the factor of the last request, `f32` bits.
    pub factor_memo: u32,
    /// Descriptor `+0x14`: the delay of the last request, `f32` bits.
    pub delay_memo: u32,
}

/// The shortest-arc wrap of `live` toward `target` that every setter does first (`0x512277` to
/// `0x5122d8`): `while target - live < -π { live -= 2π }`, then `while target - live > π { live +=
/// 2π }`. The result keeps `f64` precision; each caller rounds it to `f32` at its own stores.
fn wrap(live_bits: u32, target: f32) -> f64 {
    let mut l = f64::from(f32::from_bits(live_bits));
    let t = f64::from(target);
    while t - l < -PI {
        l -= TWO_PI;
    }
    while t - l > PI {
        l += TWO_PI;
    }
    l
}

/// The inner setter's body, shared by `0x5123c0`, `0x5125e0`, `0x512980`, `0x512ba0` and
/// `0x512dc0`: re-wrap `live`, store it as `f32`, and unless it is within 0.001 of `target` write
/// the descriptor (`from` = the wrapped live, `target`, `start_ms`, `duration`) and set the
/// channel's bit in the mode word, returning 1; within 0.001 it returns 0 with only the live word
/// updated.
///
/// `duration_bits` and `start_ms` are stored as given (`mov`), so the inner never reads them as
/// numbers: they are the dwords the outer computed.
pub fn begin_transition(
    s: &mut CamState,
    arming_bit: u32,
    target: f32,
    duration_bits: u32,
    start_ms: u32,
) -> u32 {
    let wrapped = wrap(s.live, target);
    s.live = (wrapped as f32).to_bits(); // fst [live]
    if (wrapped - f64::from(target)).abs() < DT {
        return 0;
    }
    s.from = (wrapped as f32).to_bits(); // fstp [base+0xc]
    s.target = target.to_bits(); // mov [base+0x8]
    s.start_ms = start_ms; // mov [base+0x0]
    s.flags |= arming_bit; // or [+0x90]
    s.duration = duration_bits; // mov [base+0x4]
    1
}

/// The outer setter's body, shared by `0x512270`, `0x512490`, `0x512830`, `0x512a50` and
/// `0x512c70`, called `(target, delay, factor, now)`: wrap `live` and store it as `f32`, then
///
/// 1. already arming: the channel's bit is set and the request matches the descriptor's target and
///    both memo words within 0.001 returns 1 with only the live word touched;
/// 2. already there: `|wrapped live - target| < 0.001` returns 0;
/// 3. otherwise memoise the request (`delay_memo = delay`, `factor_memo = factor`) and run the
///    inner with the start time `now - __ftol(delay * -1000)` and the duration
///    `((|target - wrapped live| / (speed * π/180)) * factor) as f32`, where `speed` is the
///    channel's speed CVar (`*(*gptr + 0x24)`).
#[allow(clippy::too_many_arguments)]
pub fn arm_transition(
    s: &mut CamState,
    arming_bit: u32,
    speed: f32,
    target: f32,
    delay: f32,
    factor: f32,
    now: i32,
) -> u32 {
    let wrapped = wrap(s.live, target);
    s.live = (wrapped as f32).to_bits(); // fst [live]

    if s.flags & arming_bit != 0
        && (f64::from(f32::from_bits(s.target)) - f64::from(target)).abs() < DT
        && (f64::from(f32::from_bits(s.delay_memo)) - f64::from(delay)).abs() < DT
        && (f64::from(f32::from_bits(s.factor_memo)) - f64::from(factor)).abs() < DT
    {
        return 1;
    }
    if (wrapped - f64::from(target)).abs() < DT {
        return 0;
    }
    s.delay_memo = delay.to_bits(); // mov [base+0x14]
    s.factor_memo = factor.to_bits(); // mov [base+0x10]
    let delay_ms = benilla_re_cmath::x87::ftol(f64::from(delay) * NEG_1000); // __ftol
    let duration = ((f64::from(target) - wrapped).abs() / (f64::from(speed) * PI_OVER_180)
        * f64::from(factor)) as f32;
    let start_ms = now.wrapping_sub(delay_ms);
    begin_transition(s, arming_bit, target, duration.to_bits(), start_ms as u32)
}

/// `CCamera::ArmFov` (`0x512270`): the field-of-view channel.
pub fn arm_fov(
    s: &mut CamState,
    speed: f32,
    target: f32,
    delay: f32,
    factor: f32,
    now: i32,
) -> u32 {
    arm_transition(s, ARM_FOV, speed, target, delay, factor, now)
}
/// `CCamera::BeginFovTransition` (`0x5123c0`).
pub fn begin_fov_transition(
    s: &mut CamState,
    target: f32,
    duration_bits: u32,
    start_ms: u32,
) -> u32 {
    begin_transition(s, ARM_FOV, target, duration_bits, start_ms)
}

/// `CCamera::ArmGroundPitch` (`0x512490`): the ground-pitch channel.
pub fn arm_ground_pitch(
    s: &mut CamState,
    speed: f32,
    target: f32,
    delay: f32,
    factor: f32,
    now: i32,
) -> u32 {
    arm_transition(s, ARM_GROUND_PITCH, speed, target, delay, factor, now)
}
/// `CCamera::BeginGroundPitchTransition` (`0x5125e0`).
pub fn begin_ground_pitch_transition(
    s: &mut CamState,
    target: f32,
    duration_bits: u32,
    start_ms: u32,
) -> u32 {
    begin_transition(s, ARM_GROUND_PITCH, target, duration_bits, start_ms)
}

/// `CCamera::ArmPitch` (`0x512830`): the pitch channel.
pub fn arm_pitch(
    s: &mut CamState,
    speed: f32,
    target: f32,
    delay: f32,
    factor: f32,
    now: i32,
) -> u32 {
    arm_transition(s, ARM_PITCH, speed, target, delay, factor, now)
}
/// `CCamera::BeginPitchTransition` (`0x512980`).
pub fn begin_pitch_transition(
    s: &mut CamState,
    target: f32,
    duration_bits: u32,
    start_ms: u32,
) -> u32 {
    begin_transition(s, ARM_PITCH, target, duration_bits, start_ms)
}

/// `CCamera::ArmPivotBias` (`0x512a50`): the pivot-bias channel.
pub fn arm_pivot_bias(
    s: &mut CamState,
    speed: f32,
    target: f32,
    delay: f32,
    factor: f32,
    now: i32,
) -> u32 {
    arm_transition(s, ARM_PIVOT_BIAS, speed, target, delay, factor, now)
}
/// `CCamera::BeginPivotBiasTransition` (`0x512ba0`).
pub fn begin_pivot_bias_transition(
    s: &mut CamState,
    target: f32,
    duration_bits: u32,
    start_ms: u32,
) -> u32 {
    begin_transition(s, ARM_PIVOT_BIAS, target, duration_bits, start_ms)
}

/// `CCamera::ArmYaw` (`0x512c70`): the yaw channel.
pub fn arm_yaw(
    s: &mut CamState,
    speed: f32,
    target: f32,
    delay: f32,
    factor: f32,
    now: i32,
) -> u32 {
    arm_transition(s, ARM_YAW, speed, target, delay, factor, now)
}
/// `CCamera::BeginYawTransition` (`0x512dc0`).
pub fn begin_yaw_transition(
    s: &mut CamState,
    target: f32,
    duration_bits: u32,
    start_ms: u32,
) -> u32 {
    begin_transition(s, ARM_YAW, target, duration_bits, start_ms)
}

// ---- the integrators ----

/// The pitch bound, 89 degrees in radians: the `f32` at `0x8089d8`, and its negation at `0x8089dc`.
const PITCH_HI: f64 = f32::from_bits(0x3fc6_d3f2) as f64;
const PITCH_LO: f64 = f32::from_bits(0xbfc6_d3f2) as f64;
/// The `f64` at `0x803090`, the `f32` 2π widened: the modulus of the angle normalizer.
const NORM_K: f64 = 6.2831854820251465;

/// The pitch integrator's clamp to ±89 degrees, in the binary's compare order: the low bound
/// first, then the high one. `f64::clamp` orders them differently and handles NaN differently.
#[allow(clippy::manual_clamp)]
fn clamp_pitch(v: f64) -> f64 {
    if v < PITCH_LO {
        PITCH_LO
    } else if PITCH_HI <= v {
        PITCH_HI
    } else {
        v
    }
}

/// Wraps `w` to within π of `m`: `while m - w < -π { w -= 2π }`, then `while π < m - w { w += 2π }`.
/// The result is stored as `f32`.
fn wrap_to(mut w: f64, m: f64) -> f32 {
    while m - w < -PI {
        w -= TWO_PI;
    }
    while PI < m - w {
        w += TWO_PI;
    }
    w as f32
}

/// The three words one integrator shifts, each stored as `f32`: the channel's target, its start
/// value and its live value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shifted {
    pub target: f32,
    pub from: f32,
    pub live: f32,
}

/// `CCamera::ShiftPitch` (`0x510120`): adds `delta` to the pitch target (`+0x1e0`), start (`+0x1e4`)
/// and live (`+0xf4`) words. The target is clamped to ±89 degrees and rounded to `f32`. The start
/// and live words are clamped the same way, then wrapped to within π of the rounded target; their
/// clamped values stay `f64` into the wrap. `delta` is added in `f64`.
pub fn shift_pitch(delta: f32, target0: f32, from0: f32, live0: f32) -> Shifted {
    let d = f64::from(delta);
    let target = clamp_pitch(d + f64::from(target0)) as f32;
    let tref = f64::from(target);
    Shifted {
        target,
        from: wrap_to(clamp_pitch(d + f64::from(from0)), tref),
        live: wrap_to(clamp_pitch(d + f64::from(live0)), tref),
    }
}

/// The angle normalizer `0x455b30`: `r = a % 2π`, plus 2π when negative. `%` on `f64` is the exact
/// IEEE remainder, which the binary's `fprem` loop also gives.
fn normalize_angle(a: f32) -> f64 {
    let mut r = f64::from(a) % NORM_K;
    if r < 0.0 {
        r += NORM_K;
    }
    r
}

/// `CCamera::ShiftYaw` (`0x5102b0`): adds `delta` to the yaw target (`+0x210`), start (`+0x214`)
/// and live (`+0xf0`) words. Each sum is rounded to `f32` (it is pushed as the argument of the
/// normalizer `0x455b30`) and normalized to `[0, 2π)`; the normalized target is rounded to `f32`,
/// and the start and live words, still `f64`, are wrapped to within π of it.
pub fn shift_yaw(delta: f32, target0: f32, from0: f32, live0: f32) -> Shifted {
    let d = f64::from(delta);
    let target = normalize_angle((d + f64::from(target0)) as f32) as f32;
    let tref = f64::from(target);
    Shifted {
        target,
        from: wrap_to(normalize_angle((d + f64::from(from0)) as f32), tref),
        live: wrap_to(normalize_angle((d + f64::from(live0)) as f32), tref),
    }
}
