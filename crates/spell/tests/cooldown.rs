//! The radius rounding, the power cost and cast time scaling, and the range window
//! (`systems/spell/records.md`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_spell::cast::INV_100;
use benilla_re_spell::cooldown::{
    spell_cast_time, spell_minmax_radius, spell_minmax_range, spell_power_cost, RangeArgs,
    PVP_RANGE_BONUS,
};
use common::*;

/// `GetSpellEffectPointsInt`, and its `call 0x6e3800` (5 bytes), which the test overwrites.
const MINMAX_RADIUS_FN: u64 = 0x6e3760;
const EFFECT_CALL_SITE: u64 = 0x6e3777;
/// The scaling of `GetSpellPowerCost`, from `fld mult` to the store at `[ebp-0xc]`.
const POWER_COST_BEGIN: u64 = 0x6e32c0;
const POWER_COST_END: u64 = 0x6e32d9;
/// The scaling of `GetSpellCastTime`, from `mov edx,[eax+0x110]` to the join at `0x6e3461`.
const CAST_TIME_BEGIN: u64 = 0x6e3426;
const CAST_TIME_END: u64 = 0x6e3461;
/// `Spell_C::GetMinMaxRange`, and its `call 0x6e6bf0` (5 bytes, after a `push 5`), which the test
/// overwrites.
const MINMAX_RANGE_FN: u64 = 0x6e3480;
const RANGE_MOD_CALL: u64 = 0x6e3746;

/// Runs `GetSpellEffectPointsInt` (`0x6e3760`) whole with its `call 0x6e3800` overwritten by `add esp,0x14`
/// (and two `nop`s), so that the bounds stay the two floats passed in the slots `GetSpellEffectPoints`
/// would fill. The function has two epilogues, so it runs whole and not as a slice. The radii
/// include negatives, values near 0, NaN, the infinities and values past the `i32` range.
#[test]
fn spell_minmax_radius_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // `add esp,0x14; nop; nop`: it pops the five dwords pushed for `0x6e3800`, which would have
    // overwritten the bounds at [ebp+0x14] and [ebp+0x18]; they keep the values passed in.
    e.write(EFFECT_CALL_SITE, &[0x83, 0xc4, 0x14, 0x90, 0x90]);
    let out_a = e.alloc(4);
    let out_b = e.alloc(4);

    // Radii the fuzz does not reach: NaN, the infinities, values past the `i32` range (where `fistp`
    // gives 0x80000000), ties at x.5, and the signed zeros.
    let probes: [(f32, f32); 11] = [
        (f32::NAN, 1.0),
        (1.0, f32::NAN),
        (f32::INFINITY, f32::NEG_INFINITY),
        (3.0e9, -3.0e9),
        (1.0e9, 2.0e9),
        (-0.0, 0.0),
        (0.25, -0.25),
        (0.5, 1.5),
        (-0.5, -1.5),
        (2.5, -2.5),
        (1.0e9, -1.0e9),
    ];
    let mut s: u32 = 0x71a3_0001;
    for i in 0..probes.len() as u32 + 8000 {
        let (radius_a, radius_b) = if let Some(&p) = probes.get(i as usize) {
            p
        } else {
            // Typical area radii (0 to 100), negatives (the `+ 0.5` form) and values near 0.
            let radius_a = match (next(&mut s) >> 8) % 3 {
                0 => fuzz_unit(&mut s, 100.0),
                1 => -fuzz_unit(&mut s, 50.0),
                _ => fuzz_unit(&mut s, 4.0) - 2.0,
            };
            let radius_b = match (next(&mut s) >> 8) % 3 {
                0 => fuzz_unit(&mut s, 100.0),
                1 => -fuzz_unit(&mut s, 50.0),
                _ => fuzz_unit(&mut s, 4.0) - 2.0,
            };
            (radius_a, radius_b)
        };
        e.write_u32(out_a, 0xdead_beef);
        e.write_u32(out_b, 0xdead_beef);
        // The stack arguments from [ebp+8]: out_a, out_b, one unused, radius_b, radius_a.
        e.call(
            MINMAX_RADIUS_FN,
            0,
            0,
            &[
                out_a as u32,
                out_b as u32,
                0,
                radius_b.to_bits(),
                radius_a.to_bits(),
            ],
        );
        let (bin_a, bin_b) = (e.read_u32(out_a) as i32, e.read_u32(out_b) as i32);
        let (our_a, our_b) = spell_minmax_radius(radius_a, radius_b);
        assert_eq!(
            (bin_a, bin_b),
            (our_a, our_b),
            "spell_minmax_radius[{i}] a={radius_a} b={radius_b}: bin=({bin_a},{bin_b}) ours=({our_a},{our_b})"
        );
    }
}

/// Runs the scaling of `GetSpellPowerCost` (`0x6e31b0`), `0x6e32c0` to `0x6e32d9`, with the unit's
/// multiplier at `[ecx + 0x2b8]`, the school 0 in `eax` and the integer cost at `[ebp-4]`, and reads
/// the result at `[ebp-0xc]`. The cases include NaN, the infinities and products out of range.
#[test]
fn spell_power_cost_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the cost at [ebp-4], the result at [ebp-0xc]
    let unit = e.alloc(0x400); // ecx: the multiplier at [ecx + 0x2b8]

    // NaN, the infinities and products past the `i32` range, which `fistp` turns into 0x80000000.
    let probes: [(f32, i32); 9] = [
        (f32::NAN, 100),
        (f32::INFINITY, 5),
        (f32::NEG_INFINITY, 5),
        (1.0e6, 22000),
        (-1.0e6, 22000),
        (-1.0, 777),
        (0.0, 0),
        (0.5, 3),
        (1.5, 1),
    ];
    let mut s: u32 = 0x4ca7_0001;
    for i in 0..probes.len() as u32 + 8000 {
        let (mult, base) = if let Some(&p) = probes.get(i as usize) {
            p
        } else {
            // The school's power cost multiplier: small, down to about -0.9, or near 0.
            let mult = match (next(&mut s) >> 8) % 3 {
                0 => fuzz_unit(&mut s, 2.0),
                1 => fuzz_unit(&mut s, 1.0) - 0.9,
                _ => fuzz_unit(&mut s, 0.001),
            };
            // The integer cost so far, from -2000 to 22000 (the caller clamps it later).
            let base = (next(&mut s) >> 8) as i32 % 24001 - 2000;
            (mult, base)
        };
        e.write_u32(unit + 0x2b8, mult.to_bits());
        e.write_u32(frame - 4, base as u32);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ECX, unit);
        e.set_reg(RegisterX86::EAX, 0); // the school, 0
        e.run(POWER_COST_BEGIN, POWER_COST_END);
        let bin = e.read_u32(frame - 0xc) as i32;
        let ours = spell_power_cost(mult, base);
        assert_eq!(
            bin, ours,
            "spell_power_cost[{i}] mult={mult} base={base}: bin={bin} ours={ours}"
        );
    }
}

/// Runs the scaling of `GetSpellCastTime` (`0x6e3340`), `0x6e3426` to `0x6e3461`, for a positive cast
/// time, with the unit's multiplier at `[unit + 0x22c]`, and reads `ecx`: the cast time left unchanged
/// when the multiplier is within 2^-22 of 1.0, else the rounded product. The multipliers cluster around
/// 1.0 and include NaN and the infinities.
#[test]
fn spell_cast_time_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the cast time at [ebp-4]
    let obj = e.alloc(0x140); // eax: the caster, its unit at +0x110
    let unit = e.alloc(0x300); // the multiplier at [unit + 0x22c]
    e.write_u32(obj + 0x110, unit as u32);

    // Multipliers at the threshold 2^-22 (4 ulp at 1.0), NaN, the infinities and a product past
    // the `i32` range.
    let probes: [(f32, i32); 13] = [
        (f32::NAN, 1000),
        (f32::INFINITY, 100),
        (f32::NEG_INFINITY, 100),
        (1.0, 100),
        (f32::from_bits(0x3f80_0001), 5000),
        (f32::from_bits(0x3f80_0002), 5000),
        (f32::from_bits(0x3f80_0003), 5000),
        (f32::from_bits(0x3f80_0004), 5000),
        (f32::from_bits(0x3f80_0005), 5000),
        (f32::from_bits(0x3f7f_fffe), 5000),
        (f32::from_bits(0x3f7f_fff8), 5000),
        (1.0e10, 1_000_000),
        (-1.0e10, 1_000_000),
    ];
    let mut s: u32 = 0x37b9_0001;
    for i in 0..probes.len() as u32 + 8000 {
        let (mult, ct) = if let Some(&p) = probes.get(i as usize) {
            p
        } else {
            // The cast speed multiplier, clustered around 1.0 to straddle the threshold 2^-22.
            let mult = match (next(&mut s) >> 8) % 4 {
                0 => fuzz_unit(&mut s, 2.0),
                1 => 1.0f32 + (fuzz_unit(&mut s, 1e-6) - 5e-7f32),
                2 => fuzz_unit(&mut s, 0.5) + 0.5,
                _ => fuzz_unit(&mut s, 3.0),
            };
            // The cast time, 1 to 600000 ms: the slice is reached only for a positive time.
            let ct = (next(&mut s) >> 8) as i32 % 600000 + 1;
            (mult, ct)
        };
        e.write_u32(unit + 0x22c, mult.to_bits());
        e.write_u32(frame - 4, ct as u32);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::EAX, obj);
        e.set_reg(RegisterX86::ECX, u64::from(ct as u32)); // the unchanged path leaves ecx as the cast time
        e.run(CAST_TIME_BEGIN, CAST_TIME_END);
        let bin = e.get_reg(RegisterX86::ECX) as u32 as i32;
        let ours = spell_cast_time(mult, ct);
        assert_eq!(
            bin, ours,
            "spell_cast_time[{i}] mult={mult} ct={ct}: bin={bin} ours={ours}"
        );
    }
}

/// Runs `Spell_C::GetMinMaxRange` (`0x6e3480`) whole with its `call 0x6e6bf0` (spell modifier 5, which
/// needs modifier tables) overwritten by `add esp,4`, over a synthesized spell record, range row and
/// units. The cases are the next-swing short-circuit, a ranged row and a melee row with a unit target.
/// The player-versus-player bonus and the ranged-item scale need unit state that is not built, so
/// `spell_minmax_range_arms_difftest` runs those two steps as slices.
#[test]
fn spell_minmax_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // `add esp,4; nop; nop`: it pops the `push 5` of the call, since min and max are final before it.
    e.write(RANGE_MOD_CALL, &[0x83, 0xc4, 0x04, 0x90, 0x90]);

    // The records and objects the function reads, the same for every case.
    let rec = e.alloc(0x100);
    let rec_table = e.alloc(0x10);
    let range_row = e.alloc(0x20);
    let range_table = e.alloc(0x10);
    let caster = e.alloc(0x200);
    let caster_unit = e.alloc(0x300);
    let caster_flags = e.alloc(0x50);
    let target = e.alloc(0x200);
    let target_desc = e.alloc(0x20);
    let target_unit = e.alloc(0x300);
    let out_min = e.alloc(4);
    let out_max = e.alloc(4);
    const SPELL_ID: u32 = 1;
    const RANGE_IDX: u32 = 1;

    // The spell store: the table at 0xc0d788, its size at 0xc0d78c.
    e.write_u32(0x00c0_d788, rec_table as u32);
    e.write_u32(0x00c0_d78c, SPELL_ID);
    e.write_u32(rec_table + 4 * u64::from(SPELL_ID), rec as u32);
    // The range store: the table at 0xc0d79c, its size at 0xc0d7a0.
    e.write_u32(0x00c0_d79c, range_table as u32);
    e.write_u32(0x00c0_d7a0, RANGE_IDX);
    e.write_u32(range_table + 4 * u64::from(RANGE_IDX), range_row as u32);
    e.write_u32(rec + 0x90, RANGE_IDX); // the record's range index
                                        // The caster: its unit at +0x110 (reach at +0x1f0), its flags at +0x118 (+0x40 clear: no
                                        // player-versus-player bonus).
    e.write_u32(caster + 0x110, caster_unit as u32);
    e.write_u32(caster + 0x118, caster_flags as u32);
    e.write_u32(caster_flags + 0x40, 0);
    // The target: its descriptor at +8 (type bit 3 at +8 makes it a unit), its unit at +0x110.
    e.write_u32(target + 8, target_desc as u32);
    e.write_u32(target_desc + 8, 0x8);
    e.write_u32(target + 0x110, target_unit as u32);

    let mut s: u32 = 0x6d11_0001;

    // The next-swing record (`+0x18 & 0x404`): the result is (0, 100.0).
    e.write_u32(rec + 0x18, 0x404);
    for i in 0..2000u32 {
        let cr = fuzz_unit(&mut s, 6.0);
        e.write_u32(caster_unit + 0x1f0, cr.to_bits());
        e.write_u32(out_min, 0xdead_beef);
        e.write_u32(out_max, 0xdead_beef);
        e.call(
            MINMAX_RANGE_FN,
            caster as u32,
            SPELL_ID,
            &[out_min as u32, out_max as u32, 0],
        );
        let (bmin, bmax) = (e.read_f32(out_min), e.read_f32(out_max));
        let (omin, omax) = spell_minmax_range(&RangeArgs {
            on_next_swing: true,
            range_row: None,
            caster_reach: cr,
            target_reach: None,
            add_reach: false,
            pvp_bonus: false,
            range_scale: None,
        });
        assert_f32(bmin, omin, &format!("range/self min[{i}]"));
        assert_f32(bmax, omax, &format!("range/self max[{i}]"));
    }

    // A ranged and a melee row, with a unit target and chosen reaches.
    e.write_u32(rec + 0x18, 0); // not next-swing, bit 1 clear: no ranged-item scale
    for i in 0..6000u32 {
        let melee = (next(&mut s) >> 8) & 1 == 1;
        let min_range = match (next(&mut s) >> 8) % 3 {
            0 => 0.0f32, // a minimum of 0, which takes no reach
            _ => fuzz_unit(&mut s, 40.0),
        };
        let max_range = fuzz_unit(&mut s, 40.0) + 5.0;
        let cr = fuzz_unit(&mut s, 7.0) + 0.1;
        let tr = fuzz_unit(&mut s, 7.0) + 0.1;
        e.write_u32(range_row + 0x4, min_range.to_bits());
        e.write_u32(range_row + 0x8, max_range.to_bits());
        e.write_u32(range_row + 0xc, u32::from(melee)); // bit 0 of +0xc is the melee flag
        e.write_u32(caster_unit + 0x1f0, cr.to_bits());
        e.write_u32(target_unit + 0x1f0, tr.to_bits());
        e.write_u32(out_min, 0xdead_beef);
        e.write_u32(out_max, 0xdead_beef);
        e.call(
            MINMAX_RANGE_FN,
            caster as u32,
            SPELL_ID,
            &[out_min as u32, out_max as u32, target as u32],
        );
        let (bmin, bmax) = (e.read_f32(out_min), e.read_f32(out_max));
        let (omin, omax) = spell_minmax_range(&RangeArgs {
            on_next_swing: false,
            range_row: Some((min_range, max_range, melee)),
            caster_reach: cr,
            target_reach: Some(tr),
            add_reach: true,
            pvp_bonus: false,
            range_scale: None,
        });
        let kind = if melee { "melee" } else { "ranged" };
        assert_f32(
            bmin,
            omin,
            &format!("range/{kind} min[{i}] minR={min_range} cr={cr} tr={tr}"),
        );
        assert_f32(
            bmax,
            omax,
            &format!("range/{kind} max[{i}] maxR={max_range} cr={cr} tr={tr}"),
        );
    }
}

/// Runs the two steps of `Spell_C::GetMinMaxRange` (`0x6e3480`) that `spell_minmax_range_difftest`
/// does not reach: the player-versus-player bonus `0x6e3697` to `0x6e36a4`, which adds 8/3 to the
/// maximum, and the ranged-item scale `0x6e3729` to `0x6e373c`, which sets the maximum to `scale ·
/// 0.01 · max`.
#[test]
fn spell_minmax_range_arms_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebx: the pointer to max at [ebx+0xc]
    let maxptr = e.alloc(4);
    let rec = e.alloc(0x200); // eax: the item record, the scale at [eax+0x118]

    let mut s: u32 = 0x33aa_0001;
    for i in 0..4000u32 {
        let max = fuzz_unit(&mut s, 80.0);
        e.write_u32(maxptr, max.to_bits());
        e.write_u32(frame + 0xc, maxptr as u32);
        e.set_reg(RegisterX86::EBX, frame);
        e.run(0x6e3697, 0x6e36a4);
        assert_f32(
            e.read_f32(maxptr),
            (f64::from(max) + f64::from(PVP_RANGE_BONUS)) as f32,
            &format!("minmax_range PvP arm[{i}] max={max}"),
        );
    }
    for i in 0..4000u32 {
        let max = fuzz_unit(&mut s, 80.0);
        let scale = fuzz_unit(&mut s, 200.0);
        e.write_u32(maxptr, max.to_bits());
        e.write_u32(rec + 0x118, scale.to_bits());
        e.write_u32(frame + 0xc, maxptr as u32);
        e.set_reg(RegisterX86::EBX, frame);
        e.set_reg(RegisterX86::EAX, rec);
        e.run(0x6e3729, 0x6e373c);
        assert_f32(
            e.read_f32(maxptr),
            (f64::from(scale) * f64::from(INV_100) * f64::from(max)) as f32,
            &format!("minmax_range range_scale arm[{i}] scale={scale} max={max}"),
        );
    }
}
