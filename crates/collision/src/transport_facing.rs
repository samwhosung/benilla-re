//! Facing arithmetic on and off a transport
//! (`systems/collision/kinematics.md#transport-space-transforms`,
//! `systems/collision/commands.md#facing-and-pitch-setters`, `systems/collision/falling.md`). The
//! transport's facing is what `GetTransportFacing` (`0x630b70`) returns in `st0`; each function
//! takes it as an argument. 2π is the `.data` value at `[0xcf5d00]` that a static initializer
//! stores.

/// `[0xcf5d00]`: 2π (`0x40c90fdb`).
pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

/// `[0x7ffd74]`: 0.0.
pub const ZERO: f32 = f32::from_bits(0x0000_0000);

/// `[0x801360]`: 0.001.
pub const MS_TO_S: f32 = f32::from_bits(0x3a83_126f);

/// `[0x8026bc]`: 2^-20.
pub const FACING_EPS: f32 = f32::from_bits(0x3580_0000);

/// One step into `[0, 2π]`: less 2π above 2π, plus 2π below 0, stored as `f32`; NaN passes.
#[inline]
fn wrap_two_pi(v: f64) -> f32 {
    let two_pi = f64::from(TWO_PI);

    if v > two_pi {
        return (v - two_pi) as f32;
    }

    if v < f64::from(ZERO) {
        (v + two_pi) as f32
    } else {
        v as f32
    }
}

/// `CMovement::TransportFacingToWorld` (`0x7c4ae0`): `wrap(transport facing + arg)`, the sum
/// unrounded; the argument unchanged with no transport.
pub fn facing_to_transport(transport_facing: f32, arg: f32, has_transport: bool) -> f32 {
    if !has_transport {
        return arg;
    }

    wrap_two_pi(f64::from(transport_facing) + f64::from(arg))
}

/// `CMovement::WorldFacingToTransport` (`0x7c4b30`): `wrap(arg − transport facing)` (`fsubr`); the
/// argument unchanged with no transport.
pub fn facing_from_transport(transport_facing: f32, arg: f32, has_transport: bool) -> f32 {
    if !has_transport {
        return arg;
    }

    wrap_two_pi(f64::from(arg) - f64::from(transport_facing))
}

/// `CMovement::GetWorldUpVector` (`0x7c4b80`), `0x7c4c2e` to `0x7c4c7c`: the matrix at `[ebp-0x40]`
/// times `+0x24`, each sum `m·x + m·z + m·y`; the three sums are stored in reverse order.
pub fn dir_to_transport_rotation(m: [f32; 16], v: [f32; 3]) -> [f32; 3] {
    let s = |off: i32| -> f64 { f64::from(m[((0x40 - off) / 4) as usize]) };
    let vx = f64::from(v[0]);
    let vy = f64::from(v[1]);
    let vz = f64::from(v[2]);

    let o2 = ((s(0x38) * vx) + (s(0x18) * vz)) + (s(0x28) * vy);

    let o1 = ((s(0x3c) * vx) + (s(0x1c) * vz)) + (s(0x2c) * vy);

    let o0 = ((s(0x40) * vx) + (s(0x20) * vz)) + (s(0x30) * vy);
    [o0 as f32, o1 as f32, o2 as f32]
}

/// `CMovement::ComputeDisplacement` (`0x7c5360`,
/// `systems/collision/kinematics.md#displacement-dispatcher`), `0x7c538a`: the step in
/// milliseconds, zero-extended to 64 bits, times 0.001, stored as `f32`.
pub fn substep_dt(ms: u32) -> f32 {
    (f64::from(ms) * f64::from(MS_TO_S)) as f32
}

/// `CMovement::TransformSpline` (`0x7c6520`), `0x7c6540`: `wrap(arg + +0x14)`.
pub fn spline_facing_accumulate(arg: f32, facing: f32) -> f32 {
    wrap_two_pi(f64::from(arg) + f64::from(facing))
}

/// `CMovement::BoardTransport` (`0x7c6840`), `0x7c684f`: the transport's facing negated, stored as
/// `f32`.
pub fn negate_transport_facing(transport_facing: f32) -> f32 {
    (-f64::from(transport_facing)) as f32
}

/// `CMovement::ReadStatusTransport` (`0x7c6900`), `0x7c6977`: `+0x80 − (b − a)` (`fsubr`), stored
/// as `f32`.
pub fn setfacingto_residual(facing_acc: f32, target_a: f32, target_b: f32) -> f32 {
    let delta = f64::from(target_b) - f64::from(target_a);
    (f64::from(facing_acc) - delta) as f32
}

/// `CMovement::SetWorldFacing` (`0x7c6eb0`): `arg − transport facing` stored as `f32`, reloaded and
/// wrapped; the facing passed to `0x7c6f30`.
pub fn setfacing_to_world(transport_facing: f32, arg: f32) -> f32 {
    let v = (f64::from(arg) - f64::from(transport_facing)) as f32;
    wrap_two_pi(f64::from(v))
}

/// `CMovement::SetFacing` (`0x7c6f30`): store unless `|new − current| < 2^-20`; NaN stores.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn setfacing_should_store(current: f32, new: f32) -> bool {
    let diff = f64::from(new) - f64::from(current);
    !(diff.abs() < f64::from(FACING_EPS))
}

/// `CMovement::SetPitch` (`0x7c6f70`): the same test on the pitch.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn setpitch_should_store(current: f32, new: f32) -> bool {
    let diff = f64::from(new) - f64::from(current);
    !(diff.abs() < f64::from(FACING_EPS))
}
