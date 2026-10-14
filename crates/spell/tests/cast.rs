//! The range gates, the placement rotation, the modifier applier and the effect amount's integer
//! formula (`systems/spell/targeting.md`, `systems/spell/checks.md`, `systems/spell/records.md`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_spell::cast::{
    spell_effect_amount, spell_mod_float, spell_objs_dist_sq, spell_range_check,
    spell_range_for_caster, spell_reticle_angle, spell_target_in_range, TWO_PI,
};
use common::*;

/// `RotateSpellPlacement`, which takes no argument and works on the facing at `0xcecad4`.
const RETICLE_ANGLE_FN: u64 = 0x6e69e0;
/// The facing the placement preview keeps.
const RETICLE_ANGLE: u64 = 0x00ce_cad4;
/// The step of `ApplySpellModifierFloat` from `fild flat` to `fstp [esi]`.
const MOD_FLOAT_BEGIN: u64 = 0x6e6c0b;
const MOD_FLOAT_END: u64 = 0x6e6c1b;

/// `Spell_C::GetMinMaxRange`, which the range tests replace by a patch that writes a chosen window.
const GET_MINMAX_RANGE: u64 = 0x6e3480;
/// `ClntObjMgrObjectPtr` (`ret 0xc`), stubbed to return a chosen object.
const OBJ_RESOLVER: u64 = 0x0046_8460;
/// The targeting flag word, with bits 0x20 and 0x40 for a location target.
const FLAG_WORD: u64 = 0x00ce_cac0;
/// The pending cast: the target guid (low and high dwords) and the spell id.
const SC_GUID_LO: u64 = 0x00ce_ac50;
const SC_GUID_HI: u64 = 0x00ce_ac54;
const SC_SPELL_ID: u64 = 0x00ce_ac58;
/// The `Spell.dbc` store (a table of record pointers by id, and its size) and the `SpellRadius.dbc`
/// store (a table of row pointers by index, and its size).
const SPELLREC_TABLE: u64 = 0x00c0_d788;
const SPELLREC_COUNT: u64 = 0x00c0_d78c;
const SPELLRADIUS_TABLE: u64 = 0x00c0_d7b0;
const SPELLRADIUS_COUNT: u64 = 0x00c0_d7b4;

/// A replacement for `Spell_C::GetMinMaxRange`: it copies the floats at `lo_slot` and `hi_slot` to the
/// two pointers passed first and second, returns 0 and pops 12 bytes, which is what both callers
/// expect. The slots are fixed and only their contents change per case, because Unicorn does not see
/// a change to code it has already translated.
fn minmax_reader_patch(lo_slot: u64, hi_slot: u64) -> [u8; 29] {
    let l = (lo_slot as u32).to_le_bytes();
    let h = (hi_slot as u32).to_le_bytes();
    [
        0x8b, 0x44, 0x24, 0x04, // mov eax,[esp+4]
        0x8b, 0x0d, l[0], l[1], l[2], l[3], // mov ecx,[lo_slot]
        0x89, 0x08, // mov [eax],ecx
        0x8b, 0x44, 0x24, 0x08, // mov eax,[esp+8]
        0x8b, 0x0d, h[0], h[1], h[2], h[3], // mov ecx,[hi_slot]
        0x89, 0x08, // mov [eax],ecx
        0x33, 0xc0, // xor eax,eax
        0xc2, 0x0c, 0x00, // ret 0xc
    ]
}

/// `IsTargetInSpellRange`.
const RANGE_CHECK_FN: u64 = 0x6e47b0;
/// `SpellTargetLocationInRange`.
const TARGET_IN_RANGE_FN: u64 = 0x6e6810;
/// `GetSpellTargetAreaRadius`.
const RANGE_FOR_CASTER_FN: u64 = 0x6e6350;
/// The three steps of `GetSpellEffectPoints` that are plain integer maths: the level delta, the
/// minimum's integer and the maximum's integer.
const EA_DELTA: (u64, u64) = (0x6e3854, 0x6e3863);
const EA_MIN: (u64, u64) = (0x6e3863, 0x6e3882);
const EA_MAX: (u64, u64) = (0x6e38bb, 0x6e38e3);
/// The squared distance of `SpellCanTargetObject`, from the loads of the two positions to the
/// store at `[ebp-0x10]`, before its call of `GetMinMaxRange`.
const OBJS_DIST_SQ_BEGIN: u64 = 0x6e6797;
const OBJS_DIST_SQ_END: u64 = 0x6e67d1;

/// Runs `RotateSpellPlacement` (`0x6e69e0`) whole on facings from 0 to past 2π and around the wrap.
#[test]
fn spell_reticle_angle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s: u32 = 0x5b17_0001;
    for i in 0..8000u32 {
        // Facings from the normal range 0 to 2π, past it, and around the wrap.
        let angle = match (next(&mut s) >> 8) % 4 {
            0 => fuzz_unit(&mut s, TWO_PI), // [0, 2π)
            1 => fuzz_unit(&mut s, 1.0) + (f64::from(TWO_PI) as f32 - 1.6), // around the 2π wrap
            2 => fuzz_unit(&mut s, 8.0),    // up to 8, past 2π
            _ => fuzz_unit(&mut s, 0.2) + 4.6, // near 2π - π/2: the sum is near 2π
        };
        e.write_u32(RETICLE_ANGLE, angle.to_bits());
        e.call(RETICLE_ANGLE_FN, 0, 0, &[]);
        let bin = e.read_f32(RETICLE_ANGLE);
        assert_f32(
            bin,
            spell_reticle_angle(angle),
            &format!("spell_reticle_angle[{i}] angle={angle}"),
        );
    }
}

/// Runs the step `0x6e6c0b` to `0x6e6c1b` of `ApplySpellModifierFloat` (`0x6e6bf0`) on chosen flat and
/// percent sums, with the value at `[esi]`.
#[test]
fn spell_mod_float_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the flat sum at [ebp+8], the percent sum at [ebp-4]
    let valptr = e.alloc(4); // esi: the value, in and out

    let mut s: u32 = 0x2c9e_0001;
    for i in 0..8000u32 {
        let flat = (next(&mut s) >> 8) as i32 % 401 - 200; // -200 to 200
        let pct = (next(&mut s) >> 8) as i32 % 401 - 100; // -100 to 300
        let value = fuzz_unit(&mut s, 5000.0);
        e.write_u32(frame + 8, flat as u32);
        e.write_u32(frame - 4, pct as u32);
        e.write_u32(valptr, value.to_bits());
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESI, valptr);
        e.run(MOD_FLOAT_BEGIN, MOD_FLOAT_END);
        let bin = e.read_f32(valptr);
        assert_f32(
            bin,
            spell_mod_float(flat, pct, value),
            &format!("spell_mod_float[{i}] flat={flat} pct={pct} value={value}"),
        );
    }
}

/// Runs `IsTargetInSpellRange` (`0x6e47b0`) whole. `Spell_C::GetMinMaxRange` is replaced by a patch
/// that writes the chosen window, and the two `GetPosition` calls are stubbed to return the chosen
/// positions. The first argument is the id, which the patch overwrites with the maximum; the second
/// is the far byte, kept in `ebx` before the patch overwrites its slot with the minimum.
#[test]
fn spell_range_check_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    let caster_obj = e.alloc(0x20);
    let target_obj = e.alloc(0x20);
    let caster_vtbl = e.alloc(0x20);
    let target_vtbl = e.alloc(0x20);
    let caster_pos = e.alloc(12);
    let target_pos = e.alloc(12);
    let island_c = e.alloc(0x10);
    let island_t = e.alloc(0x10);
    let out_byte = e.alloc(4);
    let lo_slot = e.alloc(4);
    let hi_slot = e.alloc(4);
    e.write_u32(caster_obj, caster_vtbl as u32);
    e.write_u32(target_obj, target_vtbl as u32);
    e.write_u32(caster_vtbl + 0x14, island_c as u32);
    e.write_u32(target_vtbl + 0x14, island_t as u32);
    e.stub(island_c, caster_pos as u32, 1); // GetPosition(out): thiscall, returns the position
    e.stub(island_t, target_pos as u32, 1);
    e.write(GET_MINMAX_RANGE, &minmax_reader_patch(lo_slot, hi_slot));

    // Cases at the bounds and with NaN, which compares as in range: the squared distance is exactly
    // the squared minimum and maximum, a coordinate is NaN, a bound is NaN, and the maximum is
    // infinite.
    let origin = [0.0f32; 3];
    let probes: [([f32; 3], [f32; 3], f32, f32); 11] = [
        ([3.0, 4.0, 0.0], origin, 5.0, 5.0),
        ([3.0, 4.0, 0.0], origin, 5.0, 10.0),
        ([3.0, 4.0, 0.0], origin, 1.0, 5.0),
        ([3.0, 4.0, 0.0], origin, 5.0000005, 10.0),
        ([3.0, 4.0, 0.0], origin, 1.0, 4.9999995),
        ([f32::NAN, 0.0, 0.0], origin, 1.0, 10.0),
        ([3.0, f32::NAN, 0.0], origin, 0.0, 1.0),
        ([3.0, 4.0, 0.0], origin, f32::NAN, 10.0),
        ([3.0, 4.0, 0.0], origin, 1.0, f32::NAN),
        ([3.0, 4.0, 0.0], origin, 1.0, f32::INFINITY),
        ([f32::INFINITY, 0.0, 0.0], origin, 1.0, f32::INFINITY),
    ];
    let mut s: u32 = 0x6e47_0001;
    for i in 0..probes.len() as u32 + 6000 {
        let (caster, target, min, max) = if let Some(&p) = probes.get(i as usize) {
            p
        } else {
            let caster = [
                fuzz_unit(&mut s, 200.0) - 100.0,
                fuzz_unit(&mut s, 200.0) - 100.0,
                fuzz_unit(&mut s, 50.0),
            ];
            let target = [
                caster[0] + fuzz_unit(&mut s, 80.0) - 40.0,
                caster[1] + fuzz_unit(&mut s, 80.0) - 40.0,
                caster[2] + fuzz_unit(&mut s, 40.0) - 20.0,
            ];
            let min = fuzz_unit(&mut s, 15.0);
            (caster, target, min, min + fuzz_unit(&mut s, 30.0))
        };

        write_vec3(&mut e, caster_pos, caster);
        write_vec3(&mut e, target_pos, target);
        e.write_u32(lo_slot, min.to_bits());
        e.write_u32(hi_slot, max.to_bits());
        e.write(out_byte, &[0u8; 4]);
        let eax = e.call(
            RANGE_CHECK_FN,
            caster_obj as u32,
            target_obj as u32,
            &[0, out_byte as u32],
        );
        let bin = ((eax & 0xff) != 0, e.read(out_byte, 1)[0] == 1);
        let ours = spell_range_check(caster, target, min, max);
        assert_eq!(
            bin, ours,
            "spell_range_check[{i}] caster={caster:?} target={target:?} min={min} max={max}: bin={bin:?} ours={ours:?}"
        );
    }
}

/// Runs `SpellTargetLocationInRange` (`0x6e6810`) whole with the flag word set for a location,
/// `ClntObjMgrObjectPtr` stubbed to return the caster object with a chosen position, and
/// `Spell_C::GetMinMaxRange` replaced by the patch of `minmax_reader_patch`.
#[test]
fn spell_target_in_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    let caster_obj = e.alloc(0x20);
    let caster_vtbl = e.alloc(0x20);
    let caster_pos = e.alloc(12);
    let island = e.alloc(0x10);
    let point = e.alloc(12);
    let lo_slot = e.alloc(4);
    let hi_slot = e.alloc(4);
    e.write_u32(caster_obj, caster_vtbl as u32);
    e.write_u32(caster_vtbl + 0x14, island as u32);
    e.stub(island, caster_pos as u32, 1);
    e.stub(OBJ_RESOLVER, caster_obj as u32, 3); // pops 0xc
    e.write_u32(FLAG_WORD, 0x60); // bits 5 and 6: a location target
    e.write_u32(SC_GUID_LO, 1);
    e.write_u32(SC_GUID_HI, 0);
    e.write_u32(SC_SPELL_ID, 100);
    e.write(GET_MINMAX_RANGE, &minmax_reader_patch(lo_slot, hi_slot));

    let mut s: u32 = 0x6e68_0001;
    for i in 0..6000u32 {
        let obj = [
            fuzz_unit(&mut s, 200.0) - 100.0,
            fuzz_unit(&mut s, 200.0) - 100.0,
            fuzz_unit(&mut s, 50.0),
        ];
        let pt = [
            obj[0] + fuzz_unit(&mut s, 80.0) - 40.0,
            obj[1] + fuzz_unit(&mut s, 80.0) - 40.0,
            obj[2] + fuzz_unit(&mut s, 40.0) - 20.0,
        ];
        let min = fuzz_unit(&mut s, 15.0);
        let max = min + fuzz_unit(&mut s, 30.0);

        write_vec3(&mut e, caster_pos, obj);
        write_vec3(&mut e, point, pt);
        e.write_u32(lo_slot, min.to_bits());
        e.write_u32(hi_slot, max.to_bits());
        let eax = e.call(TARGET_IN_RANGE_FN, point as u32, 1, &[]);
        let bin = eax != 0;
        let ours = spell_target_in_range(pt, obj, min, max);
        assert_eq!(
            bin, ours,
            "spell_target_in_range[{i}] point={pt:?} obj={obj:?} min={min} max={max}: bin={bin} ours={ours}"
        );
    }
}

/// Runs `GetSpellTargetAreaRadius` (`0x6e6350`) whole. The caster's level, the spell record with
/// its two radius indexes and the `SpellRadius` rows are laid out in memory; the record's `+0x280` is
/// 0, so `0x6e6b30` reports no modifier and `0x6e6bf0` leaves the value as it is. The cases cover each
/// of the two rows present and absent.
#[test]
fn spell_range_for_caster_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    let caster_obj = e.alloc(0x120);
    let unitdata = e.alloc(0x80);
    e.write_u32(caster_obj + 0x110, unitdata as u32);
    e.stub(OBJ_RESOLVER, caster_obj as u32, 3);

    let spell_id = 100u32;
    let spellrec = e.alloc(0x290);
    let rec_table = e.alloc((u64::from(spell_id) + 2) * 4);
    e.write_u32(rec_table + u64::from(spell_id) * 4, spellrec as u32);
    e.write_u32(SPELLREC_TABLE, rec_table as u32);
    e.write_u32(SPELLREC_COUNT, spell_id + 8);
    e.write_u32(SC_SPELL_ID, spell_id);
    e.write_u32(spellrec + 0x280, 0); // no modifier: `0x6e6bf0` leaves the value
    e.write_u32(SC_GUID_LO, 1);
    e.write_u32(SC_GUID_HI, 0);

    let row1 = e.alloc(0x10);
    let row2 = e.alloc(0x10);
    let rad_table = e.alloc(4 * 8);
    e.write_u32(rad_table + 4, row1 as u32); // index 1
    e.write_u32(rad_table + 8, row2 as u32); // index 2
    e.write_u32(SPELLRADIUS_TABLE, rad_table as u32);
    e.write_u32(SPELLRADIUS_COUNT, 8);

    let mut s: u32 = 0x6350_0001;
    for i in 0..6000u32 {
        let level = (next(&mut s) >> 16) as i32 % 80 + 1; // 1 to 80
        let radius1 = fuzz_unit(&mut s, 50.0);
        let mult1 = fuzz_unit(&mut s, 2.0);
        let radius2 = fuzz_unit(&mut s, 50.0);
        let mult2 = fuzz_unit(&mut s, 2.0);
        let mode = (next(&mut s) >> 8) % 4; // each of the two rows present or absent
        let r1 = if mode == 1 || mode == 3 {
            None
        } else {
            Some((radius1, mult1))
        };
        let r2 = if mode == 2 || mode == 3 {
            None
        } else {
            Some((radius2, mult2))
        };
        e.write_u32(spellrec + 0x160, if r1.is_some() { 1 } else { 0xffff_ffff });
        e.write_u32(spellrec + 0x164, if r2.is_some() { 2 } else { 0xffff_ffff });
        e.write_u32(row1 + 0x4, radius1.to_bits());
        e.write_u32(row1 + 0x8, mult1.to_bits());
        e.write_u32(row2 + 0x4, radius2.to_bits());
        e.write_u32(row2 + 0x8, mult2.to_bits());
        e.write_u32(unitdata + 0x70, level as u32);

        let bin = e.call_f32(RANGE_FOR_CASTER_FN, 0, 0, &[]);
        let ours = spell_range_for_caster(level, r1, r2);
        assert_f32(
            bin,
            ours,
            &format!("spell_range_for_caster[{i}] level={level} r1={r1:?} r2={r2:?}"),
        );
    }
}

/// Runs three stretches of `GetSpellEffectPoints` (`0x6e3800`): the level delta `0x6e3854` to
/// `0x6e3863`, the minimum's integer `0x6e3863` to `0x6e3882` and the maximum's integer `0x6e38bb` to
/// `0x6e38e3`. The software-float conversion and the modifiers after them are not run. The record
/// columns are fuzzed, including values whose products overflow 32 bits.
#[test]
fn spell_effect_amount_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let spellrec = e.alloc(0x140);
    let frame = e.alloc(0x40) + 0x20; // ebp, for the `mov [ebp-8],ecx` in the maximum's stretch

    let mut s: u32 = 0x3800_0001;
    for i in 0..8000u32 {
        let level = (next(&mut s) >> 12) as i32 % 200 - 50; // -50 to 149
        let base_level = (next(&mut s) >> 12) as i32 % 80; // 0 to 79: 0 takes the `base_level <= 0` path
        let col_100 = (next(&mut s) >> 4) as i32 % 40001 - 20000;
        let col_10c = (next(&mut s) >> 4) as i32 % 4001 - 2000;
        let col_118 = (next(&mut s) >> 4) as i32 % 4001 - 2000;
        let col_130 = (next(&mut s) >> 4) as i32 % 4001 - 2000;
        e.write_u32(spellrec + 0x70, base_level as u32);
        e.write_u32(spellrec + 0x100, col_100 as u32);
        e.write_u32(spellrec + 0x10c, col_10c as u32);
        e.write_u32(spellrec + 0x118, col_118 as u32);
        e.write_u32(spellrec + 0x130, col_130 as u32);

        // The delta: esi the record, ebx the level; the result is in ebx.
        e.set_reg(RegisterX86::ESI, spellrec);
        e.set_reg(RegisterX86::EBX, u64::from(level as u32));
        e.run(EA_DELTA.0, EA_DELTA.1);
        let delta_bin = e.get_reg(RegisterX86::EBX) as u32 as i32;

        // The minimum: esi the record, edi the effect index, ebx the delta; the result is in edx.
        e.set_reg(RegisterX86::ESI, spellrec);
        e.set_reg(RegisterX86::EDI, 0);
        e.set_reg(RegisterX86::EBX, u64::from(delta_bin as u32));
        e.set_reg(RegisterX86::EBP, frame);
        e.run(EA_MIN.0, EA_MIN.1);
        let min_bin = e.get_reg(RegisterX86::EDX) as u32 as i32;

        // The maximum, likewise.
        e.set_reg(RegisterX86::ESI, spellrec);
        e.set_reg(RegisterX86::EDI, 0);
        e.set_reg(RegisterX86::EBX, u64::from(delta_bin as u32));
        e.set_reg(RegisterX86::EBP, frame);
        e.run(EA_MAX.0, EA_MAX.1);
        let max_bin = e.get_reg(RegisterX86::EDX) as u32 as i32;

        let (min, max) = spell_effect_amount(level, base_level, col_100, col_10c, col_118, col_130);
        let mut delta_rust = level;
        if base_level > 0 {
            delta_rust = delta_rust.wrapping_sub(base_level);
        }
        if delta_rust < 0 {
            delta_rust = 0;
        }
        assert_eq!(
            delta_bin, delta_rust,
            "spell_effect_amount delta[{i}] level={level} base={base_level}"
        );
        assert_eq!(
            (min_bin, max_bin),
            (min, max),
            "spell_effect_amount[{i}] level={level} base={base_level} c100={col_100} c10c={col_10c} c118={col_118} c130={col_130}"
        );
    }
}

/// Runs the squared distance of `SpellCanTargetObject` (`0x6e6460`), `0x6e6797` to `0x6e67d1`, on two
/// chosen positions, and reads the float stored at `[ebp-0x10]`.
#[test]
fn spell_objs_dist_sq_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pos_a = e.alloc(0x10); // edi: the first position
    let pos_b = e.alloc(0x10); // eax: the second position
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80; // the result at [ebp-0x10]

    let mut s: u32 = 0x6e64_6001;
    for i in 0..8000u32 {
        // World positions: 100 either side of the origin in x and y, z from 0 to 50, and the second
        // within 80 of the first.
        let a = [
            fuzz_unit(&mut s, 200.0) - 100.0,
            fuzz_unit(&mut s, 200.0) - 100.0,
            fuzz_unit(&mut s, 50.0),
        ];
        let b = [
            a[0] + fuzz_unit(&mut s, 160.0) - 80.0,
            a[1] + fuzz_unit(&mut s, 160.0) - 80.0,
            a[2] + fuzz_unit(&mut s, 60.0) - 30.0,
        ];
        write_vec3(&mut e, pos_a, a);
        write_vec3(&mut e, pos_b, b);
        e.set_reg(RegisterX86::EDI, pos_a);
        e.set_reg(RegisterX86::EAX, pos_b);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, frame + 0x40); // room below the locals for the three argument pushes
        e.run(OBJS_DIST_SQ_BEGIN, OBJS_DIST_SQ_END);
        let bin = e.read_f32(ebp - 0x10);
        assert_f32(
            bin,
            spell_objs_dist_sq(a, b),
            &format!("spell_objs_dist_sq[{i}] a={a:?} b={b:?}"),
        );
    }
}
