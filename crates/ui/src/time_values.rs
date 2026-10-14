//! Tick counts in milliseconds that the scripts push as seconds.
//!
//! Every cooldown script, `GetTime` and `GetPlayerBuffTimeLeft` convert a count with `fild qword`
//! (the low dword is the count, the high dword is forced to 0), `fmul` by the f64 0.001 at
//! `0x801608`, and `fstp qword`; the attack time scripts multiply by the `f32` 0.001 at `0x801360`
//! instead, which is `0.0010000000474974513`. The entries are in `systems/ui/spellbook.md`,
//! `systems/ui/petbar.md`, `systems/ui/actionbar.md` and `systems/ui/auradisplay.md`.

/// `0x801608`: 0.001 rounded to an f64.
const MS_TO_S_F64: f64 = f64::from_bits(0x3f50_624d_d2f1_a9fc);
/// `0x801360`: 0.001 rounded to an `f32`.
const MS_TO_S_F32: f32 = f32::from_bits(0x3a83_126f);

/// The seconds a script pushes for a count of `ms` milliseconds, at 53 bits and not narrowed:
/// the start and the duration of `GetSpellCooldown` (`0x4b411d`, `0x4b413c`),
/// `GetShapeshiftFormCooldown`, `GetPetActionCooldown`, `GetInventoryItemCooldown` and
/// `GetActionCooldown`, `GetTime` (`0x515eb8` to `0x515ec4`) and `GetPlayerBuffTimeLeft`
/// (`0x4e497e` to `0x4e498c`).
pub fn ms_to_seconds(ms: i64) -> f64 {
    ms as f64 * MS_TO_S_F64
}

/// The attack time of `UnitRangedDamage` (`0x518987` to `0x518995`) and the two pushes of
/// `UnitAttackSpeed` (`0x518eb0` to `0x518ebe`, `0x518f34` to `0x518f42`): the same product with the
/// `f32` constant, widened.
pub fn attack_time_seconds(ms: i64) -> f64 {
    ms as f64 * f64::from(MS_TO_S_F32)
}
