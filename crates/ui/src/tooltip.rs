//! Arithmetic of the `GameTooltip` scripts and of the item tooltip builder
//! (`systems/ui/tooltip.md`, `systems/ui/tooltipitem.md`, `systems/ui/tooltipcontent.md`).

use crate::color::{pack_color, pack_gradient_color};

/// `0x808120`: 0.99999 as an `f32`, the bias that makes `__ftol` round away from zero.
const BIAS: f32 = f32::from_bits(0x3f7f_ff58);
/// `0x7ffa24`: 0.5.
const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `0x801360`: 0.001 as an `f32`.
const SCALE_001: f32 = f32::from_bits(0x3a83_126f);
/// The x87 real indefinite, `0.0 / 0.0` with exceptions masked: the sign bit is set, where the
/// host's NaN has it clear.
const X87_INDEFINITE_NAN: u64 = 0xfff8_0000_0000_0000;

use benilla_re_cmath::x87::ftol;

/// The rebase of a 1-based script index in `SetSpell` (`0x532d10`), `SetLootItem` (`0x533470`),
/// `SetMerchantItem` (`0x534080`), `SetTradePlayerItem` (`0x5341e0`), `SetTradeTargetItem`
/// (`0x534410`) and `SetMerchantCompareItem` (`0x536080`): `fsub qword [0x8015b8]`, the f64 1.0,
/// from the value `lua_tonumber` leaves in `st0`. `__ftol` then truncates it to the slot or spell
/// index.
pub fn rebase_index(arg: f64) -> f64 {
    arg - 1.0
}

/// `GameTooltip::AddLine` (`0x531630`): the line colour from arguments 3 to 5, alpha forced to
/// `0xff`.
pub fn add_line(r: f64, g: f64, b: f64) -> u32 {
    pack_gradient_color(r, g, b)
}

/// `GameTooltip::AddDoubleLine` (`0x531840`): the left colour (arguments 4 to 6) and the right
/// (7 to 9).
pub fn add_double_line(r0: f64, g0: f64, b0: f64, r1: f64, g1: f64, b1: f64) -> (u32, u32) {
    (
        pack_gradient_color(r0, g0, b0),
        pack_gradient_color(r1, g1, b1),
    )
}

/// `GameTooltip::SetText` (`0x531b90`): the colour from arguments 3 to 6, alpha 1.0 when absent.
pub fn set_text(r: f64, g: f64, b: f64, a: f64) -> u32 {
    pack_color(r, g, b, a)
}

/// A damage bound of `GetTradeSkillItemStats` (`0x4fdec0`, the minimum from `+0xb8`, slice
/// `0x4fe4ab` to `0x4fe4c6`) and of `CGTooltip::BuildItem` (`0x52b650`, `0x52c253` to `0x52c269`):
/// the float is lowered by 0.99999 when it is not positive (`fcom` against 0.0, `test ah,0x41`), then
/// `__ftol` truncates, so a non-positive value rounds toward negative infinity.
pub fn bias_round_neg(v0: f32) -> i32 {
    let v = f64::from(v0);
    ftol(if v <= 0.0 { v - f64::from(BIAS) } else { v })
}

/// The maximum of `GetTradeSkillItemStats` (`+0xcc`, `0x4fe48d` to `0x4fe4a6`) and
/// `CGTooltip::BuildItem` (`0x52c27f`): raised by 0.99999 only when positive, then truncated, so a
/// positive value rounds up.
pub fn bias_round_pos(v1: f32) -> i32 {
    let v = f64::from(v1);
    ftol(if v > 0.0 { v + f64::from(BIAS) } else { v })
}

/// `CGTooltip::BuildItem` (`0x52c2fc` and `0x52c3c5`): `fild` of the integer sum, times 0.5, stored
/// as an `f32`.
pub fn half_quantize(s20: i32, s14: i32) -> f32 {
    (f64::from(s20.wrapping_add(s14)) * f64::from(HALF)) as f32
}

/// `CGTooltip::BuildItem` (`0x52c4ce`, `0x52c595`): `fild` of the integer field `+0x110` times the
/// `f32` 0.001, at 53 bits.
pub fn scale_001(x: i32) -> f64 {
    f64::from(x) * f64::from(SCALE_001)
}

/// `CGTooltip::BuildItem` (`0x52c528` to `0x52c531`): `(a + b) * 0.5 + acc` in one x87 chain,
/// stored as an `f32`.
pub fn accumulate(a: f32, b: f32, acc: f32) -> f32 {
    ((f64::from(a) + f64::from(b)) * f64::from(HALF) + f64::from(acc)) as f32
}

/// `CGTooltip::BuildItem` (`0x52c595`, `0x52c59b`): `fdivr` divides the accumulator, a memory
/// operand, by the scaled integer in `st0`, and pushes an f64. A zero over zero is the x87 real
/// indefinite.
pub fn final_divide(x: i32, acc: f32) -> f64 {
    let q = f64::from(acc) / scale_001(x);
    if q.is_nan() {
        f64::from_bits(X87_INDEFINITE_NAN)
    } else {
        q
    }
}
