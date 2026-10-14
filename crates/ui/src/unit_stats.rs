//! `UnitAttackPower` and `UnitRangedAttackPower` (`0x518f80`, `0x5190c0`,
//! `systems/ui/unitstats.md#attack-power`).
//!
//! The x87 runs at 53-bit precision, so a chain is `f64`; `fistp` rounds to nearest even, and
//! `f32::round_ties_even` does the same where `f32::round` would round halves away from zero.

/// One term (`0x518fe2`): `fld mult; fadd [0x7ff9d8] (1.0); fimul value; fstp [m32]`, then `fld
/// [m32]; fistp [m32']`. The product is rounded to `f32` first and then to the nearest even integer.
/// `value` is the `fimul` operand: an `i32` base, or a sign-extended `i16` modifier.
pub fn attack_power_term(mult: f32, value: i32) -> i32 {
    let p = ((f64::from(mult) + 1.0) * f64::from(value)) as f32;
    p.round_ties_even() as i32
}

/// The three values `UnitAttackPower` (melee) and `UnitRangedAttackPower` push:
/// `[round((mult + 1) * base), round((mult + 1) * pos_mod), round((mult + 1) * neg_mod)]`. The
/// arithmetic is the same for both; the fields differ. The melee block is `unit+0x110` with the
/// multiplier at `+0x284` (`f32`), the base at `+0x27c` (`i32`) and the modifiers at `+0x280` and
/// `+0x282` (`i16`); the ranged fields are `0xc` higher. The caller's test `[[unit+8]+8] >> 3 & 1`
/// (`is_unit`) decides whether the fields are read; when it fails the binary pushes `0, 0, 0`.
pub fn unit_attack_power(
    mult: f32,
    base: i32,
    pos_mod: i16,
    neg_mod: i16,
    is_unit: bool,
) -> [i32; 3] {
    if !is_unit {
        return [0, 0, 0];
    }
    [
        attack_power_term(mult, base),
        attack_power_term(mult, i32::from(pos_mod)),
        attack_power_term(mult, i32::from(neg_mod)),
    ]
}
