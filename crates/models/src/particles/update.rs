//! The emitter's frame update (`systems/models/particles.md#update`): `CParticleEmitter2::Update`
//! (`0x7b5230`).

/// 2.3841858e-7 (`0x8029d4`, `0x34800000`): the least `|dt|` an update advances by.
pub const EPS: f32 = f32::from_bits(0x3480_0000);
/// 0.0 (`0x7ffd74`).
pub const CLAMP_LO: f32 = 0.0;
/// 1.0 (`0x7ff9d8`).
pub const ONE: f32 = 1.0;
/// 1/30 (`0x81d82c`, `0x3d088889`).
pub const TRAIL_PERIOD: f32 = f32::from_bits(0x3d08_8889);
/// The flag an update shorter than [`EPS`] sets (`or eax, 0x80` at `0x7b52ef`).
pub const EARLY_EXIT_FLAG: u32 = 0x80;

/// `0x7b523f..0x7b5271`: `d = matrix row 3 − camera`, each stored as `f32`, and `√((x² + y²) +
/// z²)` (`Vector3LengthSquared`, `0x4549f0`) stored as `f32` at `0xcf58e8`.
pub fn prologue_length(cur: [f32; 3], prev: [f32; 3]) -> ([f32; 3], f32) {
    let d = [0, 1, 2].map(|k| (f64::from(cur[k]) - f64::from(prev[k])) as f32);
    let [dx, dy, dz] = d.map(f64::from);
    (d, ((dx * dx + dy * dy) + dz * dz).sqrt() as f32)
}

/// `0x7b52d7..0x7b52ed`: true when `|dt| < EPS` (`test ah,5; jp`), the update that only sets
/// [`EARLY_EXIT_FLAG`]; `EPS` itself and NaN go on.
pub fn eps_gate_early_exit(arg: f32) -> bool {
    f64::from(arg).abs() < f64::from(EPS)
}

/// The flag word `+0x1ac` after the test of [`eps_gate_early_exit`].
pub fn eps_gate_flag(arg: f32, flag_in: u32) -> u32 {
    if eps_gate_early_exit(arg) {
        flag_in | EARLY_EXIT_FLAG
    } else {
        flag_in
    }
}

/// `0x7b530e..0x7b53c3`, flag 0x40000: `v = pos − last` stored as `f32` at `+0x26c`, then `r =
/// (|v|/dt)·+0x1f8 + +0x1f4` in the register, clamped to 0 below 0 (`test ah,5; jp`) and to 1
/// unless under 1 (`test ah,5; jnp`, so NaN is 1), times each component. Returns `v` and the
/// scaled `v`.
pub fn drift_block(
    pos: [f32; 3],
    prevpos: [f32; 3],
    dt: f32,
    drift_mul: f32,
    drift_add: f32,
) -> ([f32; 3], [f32; 3]) {
    let v = [0, 1, 2].map(|k| (f64::from(pos[k]) - f64::from(prevpos[k])) as f32);
    let [vx, vy, vz] = v.map(f64::from);
    let len = ((vx * vx + vy * vy) + vz * vz).sqrt();
    let r = (len / f64::from(dt)) * f64::from(drift_mul) + f64::from(drift_add);
    let clamped = if r < f64::from(CLAMP_LO) {
        f64::from(CLAMP_LO)
    } else if r < f64::from(ONE) {
        r
    } else {
        f64::from(ONE)
    };
    (v, v.map(|c| (clamped * f64::from(c)) as f32))
}

/// `0x7b53ce..0x7b54d0`, flag 0x400: `acc = dt + +0x254` stored with `fst`; only when the register
/// compares above 1/30 (`test ah,0x41; jne`, so NaN waits) the accumulator resets and `+0x258`
/// becomes `(pos − last)·(1/(acc/(1/30)))·+0x1c4`, or 0 when `+0x64` is 0. Returns the stored
/// accumulator, whether it fired, and `+0x258`.
pub fn trail_block(
    dt: f32,
    acc: f32,
    pos: [f32; 3],
    prevpos: [f32; 3],
    count: u32,
    burst: f32,
) -> (f32, bool, [f32; 3]) {
    let acc_new = f64::from(dt) + f64::from(acc);
    let period = f64::from(TRAIL_PERIOD);
    if acc_new <= period || acc_new.is_nan() {
        return (acc_new as f32, false, [0.0; 3]);
    }
    let q = acc_new / period;
    if count == 0 {
        return (0.0, true, [0.0; 3]);
    }
    let v = [0, 1, 2].map(|k| (f64::from(pos[k]) - f64::from(prevpos[k])) as f32);
    let factor = (f64::from(ONE) / q) * f64::from(burst);
    (0.0, true, v.map(|c| (factor * f64::from(c)) as f32))
}
