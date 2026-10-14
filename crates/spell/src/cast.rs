//! The range gates, the placement rotation, the modifier applier and the effect amount's integer
//! formula.

/// π/2, the float `0x3fc90fdb` at `0x808acc`: the step of the placement rotation.
pub const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
/// 2π, the float `0x40c90fdb` at `0x80295c`: where the rotation wraps.
pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
/// 0.01, the float `0x3c23d70a` at `0x8029d0`: the percent scale of the modifier appliers.
pub const INV_100: f32 = f32::from_bits(0x3c23_d70a);

/// `RotateSpellPlacement` (`0x6e69e0`): the placement facing at `0xcecad4` after one press: the
/// facing plus π/2, wrapped by 2π (`systems/spell/targeting.md#placement-model-and-rotation`).
///
/// `fst` stores the sum rounded to `f32`, but `st0` keeps the 53-bit sum, and `fcomp 2π` compares
/// that unrounded sum; the subtraction reloads the rounded store. A sum below 2π, or NaN, is kept.
pub fn spell_reticle_angle(angle: f32) -> f32 {
    let sum = f64::from(angle) + f64::from(HALF_PI);
    let a = sum as f32;
    if sum < f64::from(TWO_PI) {
        a
    } else {
        (f64::from(a) - f64::from(TWO_PI)) as f32
    }
}

/// `ApplySpellModifierFloat` (`0x6e6bf0`, the step `0x6e6c0b` to `0x6e6c1b`):
/// `(float)((flat + value) · pct · 0.01)`, each step at 53 bits, with `flat` and `pct` the sums
/// `0x6e6b30` collected (`systems/spell/records.md#modifier-appliers`).
pub fn spell_mod_float(flat: i32, pct: i32, value: f32) -> f32 {
    ((f64::from(flat) + f64::from(value)) * f64::from(pct) * f64::from(INV_100)) as f32
}

/// `IsTargetInSpellRange` (`0x6e47b0`): `(in range, far)` for a caster, a target and the range
/// window `min`, `max` that `Spell_C::GetMinMaxRange` (`0x6e3480`) gives (`systems/spell/checks.md#range-compare`).
/// The squared distance is `(dx² + dz²) + dy²` at 53 bits with `d = caster - target`. `s < min²` is
/// out of range with `far` clear; `s > max²` is out of range with `far` set; anything else, equal
/// to a bound and NaN included, is in range.
pub fn spell_range_check(caster: [f32; 3], target: [f32; 3], min: f32, max: f32) -> (bool, bool) {
    let dx = f64::from(caster[0]) - f64::from(target[0]);
    let dy = f64::from(caster[1]) - f64::from(target[1]);
    let dz = f64::from(caster[2]) - f64::from(target[2]);
    let dist_sq = (dx * dx + dz * dz) + dy * dy;
    let min2 = f64::from(min) * f64::from(min);
    let max2 = f64::from(max) * f64::from(max);
    // `!(dist_sq > max2)` and not `dist_sq <= max2`: the `jne` on `test ah,0x41` takes NaN as in range.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if dist_sq < min2 {
        (false, false)
    } else if !(dist_sq > max2) {
        (true, false)
    } else {
        (false, true)
    }
}

/// `SpellTargetLocationInRange` (`0x6e6810`): whether `min² <= d <= max²` for
/// `d = (dx² + dz²) + dy²` with `obj`, the caster's position, minus `point`, rounded to `f32`
/// (`fstp dword [ebp-0xc]`). NaN fails (`systems/spell/targeting.md#location-range-test`).
pub fn spell_target_in_range(point: [f32; 3], obj: [f32; 3], min: f32, max: f32) -> bool {
    let dx = f64::from(obj[0]) - f64::from(point[0]);
    let dy = f64::from(obj[1]) - f64::from(point[1]);
    let dz = f64::from(obj[2]) - f64::from(point[2]);
    let dist_sq = f64::from(((dx * dx + dz * dz) + dy * dy) as f32);
    let min2 = f64::from(min) * f64::from(min);
    let max2 = f64::from(max) * f64::from(max);
    dist_sq >= min2 && dist_sq <= max2
}

/// The squared distance `SpellCanTargetObject` (`0x6e6460`) stores at `[ebp-0x10]` before its range
/// compare (`0x6e6797` to `0x6e67c8`): `(dz² + dx²) + dy²` for `d = obj_b - obj_a` at 53 bits,
/// rounded to `f32`. The compare is the one of [`spell_target_in_range`]
/// (`systems/spell/targeting.md#testing-an-object-without-binding`).
#[must_use]
pub fn spell_objs_dist_sq(obj_a: [f32; 3], obj_b: [f32; 3]) -> f32 {
    let dx = f64::from(obj_b[0]) - f64::from(obj_a[0]);
    let dy = f64::from(obj_b[1]) - f64::from(obj_a[1]);
    let dz = f64::from(obj_b[2]) - f64::from(obj_a[2]);
    ((dz * dz + dx * dx) + dy * dy) as f32
}

/// `GetSpellTargetAreaRadius` (`0x6e6350`): the larger of the two level-scaled radii of the
/// spell's first two effects. A present `SpellRadius` row `(radius, per level)` gives
/// `level · per_level + radius` and an absent one gives 0.0 (`systems/spell/targeting.md#area-reticle-radius`).
/// The first is rounded to `f32`; the second stays at 53 bits. The first wins only when it is
/// greater, so a tie or NaN gives the second, rounded. The client then applies spell modifier 6
/// through `0x6e6bf0`, which leaves the value unchanged while the spell has no modifier.
pub fn spell_range_for_caster(
    level: i32,
    row1: Option<(f32, f32)>,
    row2: Option<(f32, f32)>,
) -> f32 {
    let r1: f32 = match row1 {
        Some((radius, mult)) => (f64::from(level) * f64::from(mult) + f64::from(radius)) as f32,
        None => 0.0,
    };
    let r2: f64 = match row2 {
        Some((radius, mult)) => f64::from(level) * f64::from(mult) + f64::from(radius),
        None => 0.0,
    };
    if f64::from(r1) > r2 {
        r1
    } else {
        r2 as f32
    }
}

/// The integer part of `GetSpellEffectPoints` (`0x6e3800`): `(min, max)` for the level, with
/// `delta = max(0, level - base_level)`, subtracting only a positive `base_level`
/// (`0x6e3854` to `0x6e3861`), and the effect's columns `col_N` at `+N` of its record:
///
/// - `min = col_118 · delta + col_130 + col_10c` (`0x6e3863` to `0x6e387d`)
/// - `max = col_100 · (col_118 · delta + col_10c) + col_130` (`0x6e38bb` to `0x6e38d9`)
///
/// All 32-bit wrapping. The client then converts both to software floats, adds `real · delta`
/// and applies the modifiers; those steps are not here (`systems/spell/records.md#effect-amount-formula`).
pub fn spell_effect_amount(
    level: i32,
    base_level: i32,
    col_100: i32,
    col_10c: i32,
    col_118: i32,
    col_130: i32,
) -> (i32, i32) {
    let mut delta = level;
    if base_level > 0 {
        delta = delta.wrapping_sub(base_level);
    }
    if delta < 0 {
        delta = 0;
    }
    let min = col_118
        .wrapping_mul(delta)
        .wrapping_add(col_130)
        .wrapping_add(col_10c);
    let max = col_100
        .wrapping_mul(col_118.wrapping_mul(delta).wrapping_add(col_10c))
        .wrapping_add(col_130);
    (min, max)
}
