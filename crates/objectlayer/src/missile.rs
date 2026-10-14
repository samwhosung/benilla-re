//! The missile's self-contained float math: the axis scale `0x61eec0` and the flight-speed leg of
//! `CMissile::Launch` (`0x61ceb0`) (`systems/objectlayer/missiles.md#aiming-and-leading` and
//! `#launch-and-flight-time`). The flight is linear at the spell's speed, with no gravity.
//!
//! The x87 runs at 53 bits, so an operation is an `f64` operation and each `fst dword` or `fstp
//! dword` rounds to `f32` where the code stores. A compare reads the live register.

/// The elapsed-time scale `0.001` (`0x801360`): milliseconds to seconds.
pub const TRAVELTIME_ELAPSED_SCALE: f32 = f32::from_bits(0x3a83_126f);
/// The speed cap and fallback, `100.0` (`0x80c854`): the recomputed speed is capped here, and the
/// leg uses it when no flight time remains.
pub const TRAVELTIME_MAX_SPEED: f32 = f32::from_bits(0x42c8_0000);

/// `0x61eec0(missile, out)`: `out[i] = scale * axis[i]` for the axis `(+0x5c, +0x60, +0x64)` and
/// the scale `+0x84`. Each is one `fmul` stored as `f32`; the product of two `f32` fits the 53-bit
/// mantissa, so the `f32` multiply gives the same bits.
pub fn missile_scale_vec(scale: f32, axis: [f32; 3]) -> [f32; 3] {
    [scale * axis[0], scale * axis[1], scale * axis[2]]
}

/// The flight-speed leg of `0x61ceb0`, `0x61cf9a` to `0x61d0e3`: recomputes the missile's speed
/// (`+0x1c`) so it arrives on time over the line from `origin` (`+0x20`, the launch anchor) to
/// `cur` (`+0x2c`, the current point). `now` is the `0x42c010` clock and `launch_tick` the `+0x78`
/// launch tick, whose `i32` difference is the elapsed milliseconds.
///
/// - `fcomp 0.0; test ah,0x41; jne` skips the leg unless `speed > 0`; a NaN speed also skips it,
///   and `+0x1c` is unchanged.
/// - `dist` is `sqrt((dz^2 + dy^2) + dx^2)` at 53 bits. `fst [ebp-4]` stores it as `f32` and the
///   unrounded `dist / speed` is stored as `t0`.
/// - `remaining = t0 - elapsed * 0.001`, stored as `f32` by `fst [ebp+8]` and tested live: only
///   `remaining > 0` (`test ah,0x41; je`) continues, so zero, a negative and NaN give 100.0.
/// - `q = dist_f32 / remaining_f32`; `fcomp` of 100.0 with `q` and `test ah,5; jp` keep `q` unless
///   `100 < q`, so a NaN `q` is stored.
pub fn missile_traveltime(
    origin: [f32; 3],
    cur: [f32; 3],
    speed: f32,
    now: i32,
    launch_tick: i32,
) -> f32 {
    if speed > 0.0 {
        let dx = f64::from(cur[0]) - f64::from(origin[0]);
        let dy = f64::from(cur[1]) - f64::from(origin[1]);
        let dz = f64::from(cur[2]) - f64::from(origin[2]);
        let dist = ((dz * dz + dy * dy) + dx * dx).sqrt();
        let dist_f32 = dist as f32;
        let t0 = (dist / f64::from(speed)) as f32;
        let elapsed = now.wrapping_sub(launch_tick);
        let remaining = f64::from(t0) - f64::from(elapsed) * f64::from(TRAVELTIME_ELAPSED_SCALE);
        let remaining_f32 = remaining as f32;
        if remaining > 0.0 {
            let q = f64::from(dist_f32) / f64::from(remaining_f32);
            if f64::from(TRAVELTIME_MAX_SPEED) < q {
                TRAVELTIME_MAX_SPEED
            } else {
                q as f32
            }
        } else {
            TRAVELTIME_MAX_SPEED
        }
    } else {
        speed
    }
}
