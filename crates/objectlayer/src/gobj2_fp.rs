//! Float kernels of the range test in six combat log writers: `LogUnitDeath` (`0x62c160`),
//! `LogInstakill` (`0x62cbe0`), `LogAuraDispel` (`0x62d480`) and `LogExtraAttacks` (`0x62d9f0`), which
//! subtract the positions on the stack, and `LogAuraApplication` (`0x62b800`) and `LogCastFailure`
//! (`0x62c360`), which store the differences first
//! (`systems/objectlayer/combatlog.md#the-one-sided-range-test`). The sums are those of
//! `gobj1_fp`; these return the status byte `fnstsw ax` leaves.
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fstp dword`. The positions and the
//! range come from other code; their results are inputs.

use crate::gobj1_fp::{copied_dist2, one_sided_dist2};
use crate::x87::fcom_ah;

/// The mask of `test ah,0x41`: C0 and C3, which `jne` skips the line on.
pub const TEST_AH_41: u8 = 0x41;

/// The status byte after `fcompp` of `range·range` against the squared distance in the writers that
/// subtract on the stack (`0x62c1f9` to `0x62c223`, and the same in `0x62cbe0`, `0x62d480` and
/// `0x62d9f0`): the differences stay at 53 bits and the sum is `(dx² + dz²) + dy²`.
pub fn combat_gate_inreg_ah(target: [f32; 3], player: [f32; 3], range: f32) -> u8 {
    fcom_ah(
        f64::from(range) * f64::from(range),
        one_sided_dist2(target, player),
    )
}

/// Whether those writers write their line: `test ah,0x41; jne` falls through only for `r² > d²`;
/// equal, less and NaN skip.
pub fn combat_gate_inreg_visible(target: [f32; 3], player: [f32; 3], range: f32) -> bool {
    combat_gate_inreg_ah(target, player, range) & TEST_AH_41 == 0
}

/// The status byte after `fcompp` in the writers that store the differences (`0x62b941` to
/// `0x62b95f`, and `0x62c45a` to `0x62c478` in `0x62c360`): `fstp dword` rounds each difference,
/// `0x4549a0` copies the three, and the sum of the reloaded copies is `(dx² + dy²) + dz²`. `dx`,
/// `dy` and `dz` are the stored values.
pub fn combat_gate_copied_ah(dx: f32, dy: f32, dz: f32, range: f32) -> u8 {
    fcom_ah(
        f64::from(range) * f64::from(range),
        copied_dist2(dx, dy, dz),
    )
}

/// Whether those writers write their line, as [`combat_gate_inreg_visible`].
pub fn combat_gate_copied_visible(dx: f32, dy: f32, dz: f32, range: f32) -> bool {
    combat_gate_copied_ah(dx, dy, dz, range) & TEST_AH_41 == 0
}
