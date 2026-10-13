//! Small stretches of the mover's own functions: the velocity added and removed in
//! `CMovement::IntegrateStep` (`0x616de0`, `systems/collision/mover.md#substep-loop`), the 30-yard
//! test of `CMovement::Teleport` (`0x617e90`, `systems/collision/eventapply.md#teleport-apply`),
//! the facing negation of `CMovement::SwitchTransport` (`0x618970`,
//! `systems/collision/eventapply.md#transport-attach`) and the spline fraction of
//! `CMovement::CompleteSpline` (`0x619de0`, `systems/collision/status.md#spline-completion`).

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80c730]`: 30.0.
pub const JUMP_RADIUS: f32 = f32::from_bits(0x41f0_0000);

/// 900.0, the product `fld 30.0; fmul 30.0` leaves in the register, exact.
const JUMP_RADIUS2: f64 = (JUMP_RADIUS as f64) * (JUMP_RADIUS as f64);

/// `0x618a59`: `fchs` on the transport's facing `GetTransportFacing` (`0x630b70`) returns, stored
/// as `f32`.
pub fn negate_facing(facing: f32) -> f32 {
    (-f(facing)) as f32
}

/// `0x616e74`: `delta + velocity` (`+0x44`) per component, stored as `f32`; the same code runs
/// again at `0x616ecf`.
pub fn integrate_apply_add(delta: [f32; 3], vel: [f32; 3]) -> [f32; 3] {
    [
        (f(delta[0]) + f(vel[0])) as f32,
        (f(delta[1]) + f(vel[1])) as f32,
        (f(delta[2]) + f(vel[2])) as f32,
    ]
}

/// `0x616eff`: `in − velocity` per component, stored as `f32`.
pub fn integrate_unapply_sub(inv: [f32; 3], vel: [f32; 3]) -> [f32; 3] {
    [
        (f(inv[0]) - f(vel[0])) as f32,
        (f(inv[1]) - f(vel[1])) as f32,
        (f(inv[2]) - f(vel[2])) as f32,
    ]
}

/// `0x617ebe`: `(dy² + dz²) + dx²` from the mover's position to the target's, unrounded.
pub fn facing_pos_delta2(self_pos: [f32; 3], target: [f32; 3]) -> f64 {
    let dx = f(self_pos[0]) - f(target[0]);
    let dy = f(self_pos[1]) - f(target[1]);
    let dz = f(self_pos[2]) - f(target[2]);

    (dy * dy + dz * dz) + dx * dx
}

/// `0x617ef1`: whether the squared distance is above 900 (`test ah,5; jp`); equal and NaN are not.
pub fn facing_pos_jumped(dist2: f64) -> bool {
    dist2 > JUMP_RADIUS2
}

/// `0x619e4a`: `num / denom` (`fild; fidiv`), stored as `f32`.
pub fn fall_progress(numerator: i32, denominator: i32) -> f32 {
    (f64::from(numerator) / f64::from(denominator)) as f32
}
