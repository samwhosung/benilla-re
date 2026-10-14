//! `SetDriftCloudCount` (`0x68e650`, `systems/terrain/liquid.md`): the drift cloud's particle count.

use crate::x87::ftol;

/// `[0x81038c]`, 4000.
const C_4000: f32 = f32::from_bits(0x457a_0000);

/// `0x68e650(x)` (`ecx` = the cloud, `ret 4`): stores `__ftol(x·4000)` at `+0xfa00`, the product
/// taken at 53 bits and truncated toward zero; NaN and a value beyond `i64` store 0.
pub fn drift_cloud_count(x: f32) -> i32 {
    ftol(f64::from(x) * f64::from(C_4000))
}
