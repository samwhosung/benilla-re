//! The cost, cast time and range scaling of a spell, and the integer rounding of the effect radii.

/// The float 0.5 at `0x86f94c`, which the radius rounding adds or subtracts.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);

/// `fistp dword`: the nearest integer, ties to even, or the integer indefinite `0x80000000` for
/// NaN and for a value outside the `i32` range.
fn fistp(x: f64) -> i32 {
    let r = x.round_ties_even();
    if r.is_nan() || !(-2147483648.0..2147483648.0).contains(&r) {
        i32::MIN
    } else {
        r as i32
    }
}

/// The integer bounds `GetSpellEffectPointsInt` (`0x6e3760`) returns for the two float bounds
/// `GetSpellEffectPoints` gives, `radius_a` stored first (`[ebp+0x18]`) and `radius_b`
/// (`[ebp+0x14]`) second: for each, `rint(2x - 0.5) >> 1` when `x >= 0` and `rint(2x + 0.5) >> 1`
/// otherwise, an arithmetic shift (`systems/spell/records.md#effect-amount-as-integers`).
pub fn spell_minmax_radius(radius_a: f32, radius_b: f32) -> (i32, i32) {
    (radius_round(radius_a), radius_round(radius_b))
}

fn radius_round(r: f32) -> i32 {
    let doubled = f64::from(r) + f64::from(r);
    // NaN is not `>= 0`, so it takes the `+ 0.5` form.
    let adj = if r >= 0.0 {
        doubled - f64::from(HALF)
    } else {
        doubled + f64::from(HALF)
    };
    fistp(adj) >> 1
}

/// The cast-speed threshold 2^-22, the float `0x34800000` at `0x8029d4`.
pub const CAST_TIME_EPSILON: f32 = f32::from_bits(0x3480_0000);
/// 5.0, the melee range floor (`0x80a1e8`).
pub const MELEE_FLOOR: f32 = 5.0;
/// 100.0, the maximum range of a spell with `+0x18 & 0x404` (`0x8118d4`).
pub const ON_NEXT_SWING_RANGE: f32 = 100.0;
/// 4/3, the float `0x3faaaaab` (`0x80b058`), added to the melee reach.
pub const COMBAT_REACH_ADD: f32 = f32::from_bits(0x3faa_aaab);
/// 8/3, the float `0x402aaaab` (`0x811914`), added to the maximum range of a player-versus-player cast.
pub const PVP_RANGE_BONUS: f32 = f32::from_bits(0x402a_aaab);

/// The scaling in `GetSpellPowerCost` (`0x6e31b0`, the step `0x6e32c0` to `0x6e32d9`):
/// `rint((float)((mult + 1.0) · base))`, with `mult` the unit's float at `+0x2b8` for the school and
/// `base` the integer cost so far. The product is at 53 bits, `fstp` rounds it to `f32`, and
/// `fistp` rounds to nearest-even; NaN and values out of range give `0x80000000`, which the caller's
/// clamp makes 0 (`systems/spell/records.md#power-cost`).
pub fn spell_power_cost(mult: f32, base: i32) -> i32 {
    let prod = ((f64::from(mult) + 1.0) * f64::from(base)) as f32;
    fistp(f64::from(prod))
}

/// The scaling in `GetSpellCastTime` (`0x6e3340`, the step `0x6e3426` to `0x6e3461`), reached for
/// `ct > 0`: with `mult` the unit's cast-speed float at `+0x22c`, `ct` is unchanged when
/// `|mult - 1| < 2^-22` and otherwise `rint((float)(mult · ct))`. The difference is at 53 bits; a
/// NaN `mult` multiplies, and gives `0x80000000` (`systems/spell/records.md#cast-time`).
pub fn spell_cast_time(mult: f32, ct: i32) -> i32 {
    let diff = (f64::from(mult) - 1.0).abs();
    if diff < f64::from(CAST_TIME_EPSILON) {
        ct
    } else {
        let prod = (f64::from(mult) * f64::from(ct)) as f32;
        fistp(f64::from(prod))
    }
}

/// What `Spell_C::GetMinMaxRange` (`0x6e3480`) reads before its arithmetic, resolved
/// (`systems/spell/records.md#range`, `systems/spell/records.md#range-bonuses-and-modifier`).
pub struct RangeArgs {
    /// `SpellRec+0x18 & 0x404`: the range is `(0, 100.0)` and nothing else is read.
    pub on_next_swing: bool,
    /// The `SpellRange` row `(min at +0x04, max at +0x08, melee: +0x0c bit 0)`, or `None` when
    /// `RangeIndex` (`+0x90`) is out of range or the row is null; the range is then `(0, 0)`
    /// before the ranged-item step.
    pub range_row: Option<(f32, f32, bool)>,
    /// The caster's reach, the float at `+0x1f0` of its unit data.
    pub caster_reach: f32,
    /// The reach of the unit in `esi`, or `None` when there is none. The unit is the target
    /// argument when its type word has bit 3; otherwise, in the melee arm only, the unit whose
    /// guid is at `+0xc48` of the caster (`0x6e3552` to `0x6e3584`). With `None` the caster's
    /// reach stands in as the second term.
    pub target_reach: Option<f32>,
    /// The ranged arm adds reach: the target argument is present and its type has bit 3 or 7
    /// (`0x6e35ee`). The arm has no fallback to the auto-attack unit.
    pub add_reach: bool,
    /// The player-versus-player bonus applies (`0x6e3648` to `0x6e36a2`): a unit is in `esi`, both
    /// units' flags at `+0x118+0x40` have a bit of `0x200d` with `0x5fc350` returning 0 for both,
    /// and the range row has the melee bit or the unit's type has bit 4.
    pub pvp_bonus: bool,
    /// `W`, the float at `+0x118` of the item record in the caster's ranged slot, when
    /// `SpellRec+0x18` has bit 1, the caster is a player and the item is cached
    /// (`0x6e36aa` to `0x6e373a`): `max` becomes `W · 0.01 · max`. `None` leaves `max` unchanged.
    pub range_scale: Option<f32>,
}

/// The arithmetic of `Spell_C::GetMinMaxRange` (`0x6e3480`) over resolved [`RangeArgs`], giving
/// `(min, max)` before the final spell modifier 5 (`0x6e6bf0`):
///
/// - On the next swing: `(0, 100.0)`.
/// - A melee row: `min = 0`; `sum = reach + caster_reach + 4/3` at 53 bits; `max = sum` rounded to
///   `f32` if `sum > 5.0`, else 5.0 (NaN gives 5.0).
/// - Another row: `min = row min`, `max = row max`; with `add_reach`, `reach_sum = caster_reach +
///   reach` at 53 bits, `max = (row max + reach_sum) as f32` and, only if the row's min is not 0,
///   `min = (reach_sum + row min) as f32`.
/// - The player-versus-player bonus `max = (max + 8/3) as f32`, then the ranged-item scale
///   `max = (W · 0.01 · max) as f32`.
///
/// `reach` is `target_reach` when a unit is in `esi`, else `caster_reach`.
pub fn spell_minmax_range(a: &RangeArgs) -> (f32, f32) {
    if a.on_next_swing {
        return (0.0, ON_NEXT_SWING_RANGE);
    }
    let mut min = 0.0f32;
    let mut max = 0.0f32;
    if let Some((row_min, row_max, melee)) = a.range_row {
        let reach = a.target_reach.unwrap_or(a.caster_reach);
        if melee {
            min = 0.0;
            let sum53 = f64::from(reach) + f64::from(a.caster_reach) + f64::from(COMBAT_REACH_ADD);
            let sum = sum53 as f32;
            max = if sum53 > f64::from(MELEE_FLOOR) {
                sum
            } else {
                MELEE_FLOOR
            };
        } else {
            min = row_min;
            max = row_max;
            if a.add_reach {
                let reach_sum = f64::from(a.caster_reach) + f64::from(reach);
                max = (f64::from(row_max) + reach_sum) as f32;
                if row_min != 0.0 {
                    min = (reach_sum + f64::from(row_min)) as f32;
                }
            }
        }
        if a.target_reach.is_some() && a.pvp_bonus {
            max = (f64::from(max) + f64::from(PVP_RANGE_BONUS)) as f32;
        }
    }
    if let Some(scale) = a.range_scale {
        max = (f64::from(scale) * f64::from(crate::cast::INV_100) * f64::from(max)) as f32;
    }
    (min, max)
}
